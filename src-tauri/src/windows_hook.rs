//! Windows-only: prevent Alt+Space from opening the system menu, and
//! conditionally swallow Escape to close the quick-switcher.
//!
//! We install TWO complementary mechanisms because each has its own failure
//! mode:
//!
//! 1. **`WH_KEYBOARD_LL`** (system-wide low-level keyboard hook): swallows
//!    Alt+Space at the raw input layer before any window's queue sees it,
//!    and conditionally swallows Escape to close the quick-switcher when
//!    it has lost focus (so the focused window — typically a tool window —
//!    doesn't intercept the keystroke for its own purposes first).
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
//!
//! ## Why a low-level hook for Escape instead of a global shortcut
//!
//! `tauri-plugin-global-shortcut` uses `RegisterHotKey`, which consumes
//! the keystroke at the OS level — Escape would never reach the focused
//! webview, so tool windows' own Escape handlers (e.g. close on Esc) would
//! stop working. The LL hook, by contrast, can `CallNextHookEx` to let the
//! keystroke continue through to the focused window when our condition
//! isn't met.

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

/// Virtual key code for Escape.
const VK_ESCAPE: u32 = 0x1B;

/// `KBDLLHOOKSTRUCT::flags` bit set while the ALT key is held down.
const LLKHF_ALTDOWN: u32 = 0x20;

/// `ShowWindow` nCmdShow: hide the window and activate another.
const SW_HIDE: i32 = 0;

/// `SetWindowLongPtrW` index for the window procedure pointer.
const GWLP_WNDPROC: i32 = -4;

/// `SetWindowLongPtrW` index for the window style bits.
const GWL_STYLE: i32 = -16;

/// `WS_SYSMENU` — having this style bit is what makes a window respond to
/// Alt+Space by opening the system menu. Stripping it makes Alt+Space a no-op
/// for that window. Safe for decoration-less windows (quick switcher) where
/// the user can't see / interact with the system menu anyway.
const WS_SYSMENU: isize = 0x00080000;

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
    fn GetWindowLongPtrW(hwnd: isize, n_index: i32) -> isize;
    fn SetWindowLongPtrW(hwnd: isize, n_index: i32, dw_new_long: isize) -> isize;
    fn CallWindowProcW(
        prev_wnd_func: isize,
        hwnd: isize,
        msg: u32,
        w_param: usize,
        l_param: isize,
    ) -> isize;
    fn SetPropW(hwnd: isize, lp_string: *const u16, h_data: isize) -> i32;
    fn GetPropW(hwnd: isize, lp_string: *const u16) -> isize;
    fn IsWindowVisible(hwnd: isize) -> i32;
    fn GetForegroundWindow() -> isize;
    fn ShowWindow(hwnd: isize, n_cmd_show: i32) -> i32;
}

type HOOKPROC = unsafe extern "system" fn(i32, usize, isize) -> isize;

/// 0 means "not installed".
static HOOK_HANDLE: AtomicIsize = AtomicIsize::new(0);

/// Only log the FIRST intercept of each path to avoid spamming stderr (which
/// could starve the LL hook of its timing budget).
static LOGGED_LL_INTERCEPT: AtomicBool = AtomicBool::new(false);
static LOGGED_SC_INTERCEPT: AtomicBool = AtomicBool::new(false);
static LOGGED_ESC_INTERCEPT: AtomicBool = AtomicBool::new(false);

/// When `false`, the hook & subclass let Alt+Space through so the settings UI
/// can capture it for shortcut recording. Set via [`set_suppress`].
static SUPPRESS: AtomicBool = AtomicBool::new(true);

/// HWND of the pre-created quick-switcher window, or 0 if not yet known.
/// Set by [`set_qs_hwnd`] during quick-switcher pre-creation. The LL hook
/// reads this on every ESC to decide whether to swallow-and-hide.
static QS_HWND: AtomicIsize = AtomicIsize::new(0);

/// Record the HWND of the pre-created quick-switcher window so the LL hook
/// can swallow Escape and hide it even when QS has lost focus. Idempotent.
pub fn set_qs_hwnd(hwnd: isize) {
    QS_HWND.store(hwnd, Ordering::Relaxed);
}

/// Wide-string key under which the original `WNDPROC` for each subclassed
/// HWND is stored via `SetPropW`. Lets a single `subclass_proc` serve many
/// windows — it looks up the per-HWND original procedure on each message.
const ORIG_PROP: &[u16] = &[
    b'T' as u16,
    b'o' as u16,
    b'o' as u16,
    b'l' as u16,
    b'B' as u16,
    b'e' as u16,
    b'n' as u16,
    b'c' as u16,
    b'h' as u16,
    b'O' as u16,
    b'r' as u16,
    b'i' as u16,
    b'g' as u16,
    b'W' as u16,
    b'n' as u16,
    b'd' as u16,
    b'P' as u16,
    b'r' as u16,
    b'o' as u16,
    b'c' as u16,
    0,
];

/// Toggle whether Alt+Space gets swallowed. Pass `false` while the user is
/// recording a new shortcut so the keystroke can reach the webview.
pub fn set_suppress(enabled: bool) {
    SUPPRESS.store(enabled, Ordering::Relaxed);
}

unsafe extern "system" fn keyboard_proc(n_code: i32, w_param: usize, l_param: isize) -> isize {
    if n_code == HC_ACTION {
        let kb = &*(l_param as *const KBDLLHOOKSTRUCT);

        // Alt+Space — swallow so it doesn't pop the system menu.
        if w_param == WM_SYSKEYDOWN as usize
            && kb.vk_code == VK_SPACE
            && (kb.flags & LLKHF_ALTDOWN) != 0
            && SUPPRESS.load(Ordering::Relaxed)
        {
            if !LOGGED_LL_INTERCEPT.swap(true, Ordering::Relaxed) {
                eprintln!("[windows_hook] swallowed Alt+Space via WH_KEYBOARD_LL");
            }
            return 1;
        }

        // Escape — conditionally swallow to close the quick-switcher.
        //
        // Only swallow when ALL of:
        //   1. The quick-switcher is registered (QS_HWND != 0)
        //   2. The QS window is currently visible (IsWindowVisible)
        //   3. The QS window does NOT have OS-level focus — otherwise the
        //      webview is the one that should receive the key, and our
        //      JS-side listener in QuickSwitcher.tsx handles it.
        //
        // This is the difference vs. a RegisterHotKey-based global
        // shortcut: we let Escape continue to the focused window whenever
        // the condition isn't met, so tool windows' own Escape handlers
        // (e.g. close on Esc) keep working.
        if kb.vk_code == VK_ESCAPE {
            let qs_hwnd = QS_HWND.load(Ordering::Relaxed);
            if qs_hwnd != 0
                && IsWindowVisible(qs_hwnd) != 0
                && GetForegroundWindow() != qs_hwnd
            {
                ShowWindow(qs_hwnd, SW_HIDE);
                if !LOGGED_ESC_INTERCEPT.swap(true, Ordering::Relaxed) {
                    eprintln!("[windows_hook] swallowed ESC and hid QS (HWND=0x{qs_hwnd:x})");
                }
                return 1;
            }
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
    if msg == WM_SYSCOMMAND
        && (w_param & 0xFFF0) == SC_KEYMENU
        && SUPPRESS.load(Ordering::Relaxed)
    {
        if !LOGGED_SC_INTERCEPT.swap(true, Ordering::Relaxed) {
            eprintln!("[windows_hook] swallowed SC_KEYMENU via window subclass HWND=0x{hwnd:x}");
        }
        return 0;
    }
    let orig = GetPropW(hwnd, ORIG_PROP.as_ptr());
    if orig != 0 {
        CallWindowProcW(orig, hwnd, msg, w_param, l_param)
    } else {
        // Should never happen — we only install this proc via `subclass`
        // which sets the prop atomically. Returning 0 is the safest fallback.
        0
    }
}

/// Install both the LL keyboard hook and a subclass on the main window.
/// Additional windows (quick switcher, tool windows, ...) should call
/// [`subclass`] themselves right after they're created.
pub fn install(app: &AppHandle) {
    install_ll_hook();
    if let Some(window) = app.get_webview_window("main") {
        if let Ok(hwnd) = window.hwnd() {
            subclass(hwnd.0 as isize);
        }
    } else {
        eprintln!("[windows_hook] main window not found at install time");
    }
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

/// Subclass an arbitrary HWND so its `WM_SYSCOMMAND` / `SC_KEYMENU`
/// (Alt+Space-triggered system menu) gets swallowed. Idempotent — a second
/// call for the same HWND is a no-op.
pub fn subclass(hwnd: isize) {
    if hwnd == 0 {
        return;
    }
    unsafe {
        // Already subclassed by us? Then bail.
        if GetPropW(hwnd, ORIG_PROP.as_ptr()) != 0 {
            return;
        }
        let orig = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, subclass_proc as *const () as isize);
        if orig == 0 {
            eprintln!("[windows_hook] subclass: SetWindowLongPtrW returned 0 for HWND=0x{hwnd:x}");
            return;
        }
        // Stash the original wndproc on the window itself so `subclass_proc`
        // can find it via `GetPropW` (lets the same proc serve many HWNDs).
        if SetPropW(hwnd, ORIG_PROP.as_ptr(), orig) == 0 {
            eprintln!("[windows_hook] subclass: SetPropW failed for HWND=0x{hwnd:x}");
            // Restore — better to leak the subclass than to lose the orig.
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, orig);
            return;
        }
        eprintln!("[windows_hook] subclassed HWND=0x{hwnd:x}");
    }
}

/// Strip `WS_SYSMENU` from a window so Alt+Space stops opening the system
/// menu for it. Only safe for windows that are decoration-less or otherwise
/// don't need a system menu. Less reliable than [`subclass`] — some window
/// toolkits (e.g. Tao) reset window styles after creation, so a freshly
/// stripped HWND may grow `WS_SYSMENU` back. Prefer [`subclass`].
pub fn disable_sysmenu(hwnd: isize) {
    if hwnd == 0 {
        return;
    }
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        if style == 0 {
            return;
        }
        if (style & WS_SYSMENU) != 0 {
            SetWindowLongPtrW(hwnd, GWL_STYLE, style & !WS_SYSMENU);
            eprintln!("[windows_hook] WS_SYSMENU stripped from HWND=0x{hwnd:x}");
        }
    }
}
