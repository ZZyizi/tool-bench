//! Windows-only: prevent Alt+Space from opening the system menu.
//!
//! We install TWO complementary mechanisms because each has its own failure
//! mode:
//!
//! 1. **`WH_KEYBOARD_LL`** (system-wide low-level keyboard hook): swallows
//!    Alt+Space at the raw input layer before any window's queue sees it.
//!    Critical: the hook proc MUST be fast. If it consistently exceeds
//!    `LowLevelHooksTimeout` (default 300 ms), Windows silently disables it
//!    and subsequent keystrokes bypass our filter. So we do **not** log
//!    inside the hot path — only the first intercept is recorded.
//!
//! 2. **Window subclassing** on the main Tauri (Tao) parent window: catches
//!    the `WM_SYSCOMMAND` with `SC_KEYMENU` payload that `DefWindowProc`
//!    generates when Alt+Space goes unhandled. This is the fallback if the
//!    LL hook fails to catch Space — Alt+Space goes to the focused WebView2
//!    child, its `DefWindowProc` posts `WM_SYSCOMMAND(SC_KEYMENU)` to the
//!    top-level parent, and we swallow it there.

use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

use tauri::{AppHandle, Manager};

// ---- Win32 constants -----------------------------------------------------

/// `SetWindowsHookEx` idHook for a low-level keyboard hook.
const WH_KEYBOARD_LL: i32 = 13;

/// Hook proc is called with `nCode == HC_ACTION` for actionable events.
const HC_ACTION: i32 = 0;

/// Alt+key sends `WM_SYSKEYDOWN` instead of `WM_KEYDOWN`.
const WM_SYSKEYDOWN: u32 = 0x0104;

/// `DefWindowProc` posts this to the top-level window for system menu commands.
const WM_SYSCOMMAND: u32 = 0x0112;

/// `wParam` of `WM_SYSCOMMAND` (masked with `0xFFF0`) when Alt+Space is pressed.
const SC_KEYMENU: usize = 0xF100;

/// Virtual key code for the spacebar.
const VK_SPACE: u32 = 0x20;

/// `KBDLLHOOKSTRUCT::flags` bit set while the ALT key is held down.
const LLKHF_ALTDOWN: u32 = 0x20;

/// `SetWindowLongPtrW` index for the window procedure pointer.
const GWLP_WNDPROC: i32 = -4;

#[repr(C)]
struct KBDLLHOOKSTRUCT {
    vk_code: u32,
    scan_code: u32,
    flags: u32,
    time: u32,
    dw_extra_info: usize,
}

extern "system" {
    fn SetWindowsHookExW(
        id_hook: i32,
        lpfn: HOOKPROC,
        hmod: isize,
        dw_thread_id: u32,
    ) -> isize;
    fn CallNextHookEx(hhk: isize, n_code: i32, w_param: usize, l_param: isize) -> isize;
    fn GetModuleHandleW(lp_module_name: *const u16) -> isize;
    fn SetWindowLongPtrW(hwnd: isize, n_index: i32, dw_new_long: isize) -> isize;
    fn CallWindowProcW(
        prev_wnd_func: isize,
        hwnd: isize,
        msg: u32,
        w_param: usize,
        l_param: isize,
    ) -> isize;
}

type HOOKPROC = unsafe extern "system" fn(i32, usize, isize) -> isize;

/// 0 means "not installed".
static HOOK_HANDLE: AtomicIsize = AtomicIsize::new(0);
static ORIG_WNDPROC: AtomicIsize = AtomicIsize::new(0);

/// Only log the FIRST intercept of each path to avoid spamming stderr (which
/// could starve the LL hook of its timing budget).
static LOGGED_LL_INTERCEPT: AtomicBool = AtomicBool::new(false);
static LOGGED_SC_INTERCEPT: AtomicBool = AtomicBool::new(false);

unsafe extern "system" fn keyboard_proc(n_code: i32, w_param: usize, l_param: isize) -> isize {
    if n_code == HC_ACTION && w_param == WM_SYSKEYDOWN as usize {
        let kb = &*(l_param as *const KBDLLHOOKSTRUCT);
        if kb.vk_code == VK_SPACE && (kb.flags & LLKHF_ALTDOWN) != 0 {
            if !LOGGED_LL_INTERCEPT.swap(true, Ordering::Relaxed) {
                eprintln!("[windows_hook] swallowed Alt+Space via WH_KEYBOARD_LL");
            }
            // Nonzero swallows the keystroke before any window sees it.
            return 1;
        }
    }
    CallNextHookEx(HOOK_HANDLE.load(Ordering::Relaxed), n_code, w_param, l_param)
}

unsafe extern "system" fn subclass_proc(
    hwnd: isize,
    msg: u32,
    w_param: usize,
    l_param: isize,
) -> isize {
    if msg == WM_SYSCOMMAND && (w_param & 0xFFF0) == SC_KEYMENU {
        if !LOGGED_SC_INTERCEPT.swap(true, Ordering::Relaxed) {
            eprintln!("[windows_hook] swallowed SC_KEYMENU via window subclass");
        }
        return 0;
    }
    let orig = ORIG_WNDPROC.load(Ordering::Relaxed);
    if orig != 0 {
        CallWindowProcW(orig, hwnd, msg, w_param, l_param)
    } else {
        0
    }
}

/// Install both the LL keyboard hook and the window subclass.
pub fn install(app: &AppHandle) {
    install_ll_hook();
    install_window_subclass(app);
}

fn install_ll_hook() {
    if HOOK_HANDLE.load(Ordering::Relaxed) != 0 {
        return;
    }
    unsafe {
        // System-wide LL hook with the EXE module handle — the only form that
        // works when the proc lives in the EXE (vs. a DLL).
        let hmod = GetModuleHandleW(std::ptr::null());
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, keyboard_proc as HOOKPROC, hmod, 0);
        if hook != 0 {
            HOOK_HANDLE.store(hook, Ordering::Relaxed);
            eprintln!("[windows_hook] WH_KEYBOARD_LL installed");
        } else {
            eprintln!("[windows_hook] WH_KEYBOARD_LL install failed");
        }
    }
}

fn install_window_subclass(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        eprintln!("[windows_hook] subclass: main window not found");
        return;
    };
    let hwnd = match window.hwnd() {
        Ok(h) => h.0 as isize,
        Err(e) => {
            eprintln!("[windows_hook] subclass: failed to get HWND: {e}");
            return;
        }
    };
    if hwnd == 0 {
        eprintln!("[windows_hook] subclass: HWND is null");
        return;
    }
    unsafe {
        let orig = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, subclass_proc as *const () as isize);
        if orig != 0 {
            ORIG_WNDPROC.store(orig, Ordering::Relaxed);
            eprintln!("[windows_hook] subclassed main window HWND=0x{hwnd:x}");
        } else {
            eprintln!("[windows_hook] SetWindowLongPtrW returned 0");
        }
    }
}
