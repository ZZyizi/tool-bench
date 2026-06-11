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
//!    and subsequent keystrokes bypass our filter. **No logging or blocking
//!    calls inside the hook proc.** All diagnostics are exposed via atomic
//!    counters read by the `get_hook_diagnostics` Tauri command.
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

use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering};

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

/// Regular key-down message.
const WM_KEYDOWN: u32 = 0x0100;

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

/// `WM_USER` is the start of the range of messages that are defined by
/// application code. We use `WM_USER + 1` to ask the QS window's wndproc
/// (subclassed, runs on the main thread) to hide the window.
const WM_USER: u32 = 0x0400;
const QS_HIDE_MSG: u32 = WM_USER + 1;

/// `WS_SYSMENU` — having this style bit is what makes a window respond to
/// Alt+Space by opening the system menu.
const WS_SYSMENU: isize = 0x00080000;

#[repr(C)]
#[allow(clippy::upper_case_acronyms)] // mirrors the Win32 SDK name
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
    fn PostMessageW(hwnd: isize, msg: u32, w_param: usize, l_param: isize) -> i32;
}

#[allow(clippy::upper_case_acronyms)] // mirrors the Win32 SDK name
type HOOKPROC = unsafe extern "system" fn(i32, usize, isize) -> isize;

/// 0 means "not installed".
static HOOK_HANDLE: AtomicIsize = AtomicIsize::new(0);

// ---- Diagnostic counters (all hot-path safe: atomic fetch_add is ~ns) ----

/// Every call to keyboard_proc with HC_ACTION.
static LL_HOOK_HITS: AtomicU64 = AtomicU64::new(0);
/// Escape key seen by the hook AND QS is visible.
static LL_HOOK_ESC_HITS: AtomicU64 = AtomicU64::new(0);
/// Escape let through to the webview (QS had foreground focus).
static LL_ESC_PASSTHROUGH: AtomicU64 = AtomicU64::new(0);
/// Escape swallowed + PostMessage to subclass (QS visible but not focused).
static LL_ESC_POSTMSG: AtomicU64 = AtomicU64::new(0);
/// Escape seen but QS_HWND==0 or not visible.
static LL_ESC_SKIP: AtomicU64 = AtomicU64::new(0);

/// Only log the FIRST intercept of `WM_SYSCOMMAND / SC_KEYMENU` (subclass
/// path — runs on main thread, no timeout risk).
static LOGGED_SC_INTERCEPT: AtomicBool = AtomicBool::new(false);

/// When `false`, the hook & subclass let Alt+Space through so the settings UI
/// can capture it for shortcut recording.
static SUPPRESS: AtomicBool = AtomicBool::new(true);

/// HWND of the pre-created quick-switcher window, or 0 if not yet known.
static QS_HWND: AtomicIsize = AtomicIsize::new(0);

// ---- Public API -----------------------------------------------------------

/// Record the HWND of the pre-created quick-switcher window.
pub fn set_qs_hwnd(hwnd: isize) {
    let prev = QS_HWND.swap(hwnd, Ordering::Relaxed);
    let hits = LL_HOOK_HITS.load(Ordering::Relaxed);
    let esc = LL_HOOK_ESC_HITS.load(Ordering::Relaxed);
    let installed = HOOK_HANDLE.load(Ordering::Relaxed) != 0;
    eprintln!(
        "[windows_hook] QS_HWND=0x{hwnd:x} (was 0x{prev:x}) installed={installed} hits={hits} esc={esc}"
    );
}

/// Toggle whether Alt+Space gets swallowed. Pass `false` while the user is
/// recording a new shortcut so the keystroke can reach the webview.
pub fn set_suppress(enabled: bool) {
    SUPPRESS.store(enabled, Ordering::Relaxed);
}

/// Return all diagnostic counters from the LL hook. Safe to call from the
/// frontend via `invoke("get_hook_diagnostics")`.
#[tauri::command]
pub fn get_hook_diagnostics() -> serde_json::Value {
    let installed = HOOK_HANDLE.load(Ordering::Relaxed) != 0;
    let qs_hwnd = QS_HWND.load(Ordering::Relaxed);
    let qs_visible = if qs_hwnd != 0 {
        unsafe { IsWindowVisible(qs_hwnd) != 0 }
    } else {
        false
    };
    let fg = unsafe { GetForegroundWindow() };
    let qs_has_focus = qs_hwnd != 0 && fg == qs_hwnd;

    serde_json::json!({
        "installed": installed,
        "qs_hwnd": qs_hwnd,
        "qs_visible": qs_visible,
        "qs_has_focus": qs_has_focus,
        "foreground_hwnd": fg,
        "total_hits": LL_HOOK_HITS.load(Ordering::Relaxed),
        "esc_hits": LL_HOOK_ESC_HITS.load(Ordering::Relaxed),
        "esc_passthrough": LL_ESC_PASSTHROUGH.load(Ordering::Relaxed),
        "esc_postmsg": LL_ESC_POSTMSG.load(Ordering::Relaxed),
        "esc_skip": LL_ESC_SKIP.load(Ordering::Relaxed),
    })
}

// ---- Low-level keyboard hook ----------------------------------------------
//
// CRITICAL: zero `eprintln!` / blocking calls inside this function.
// A single console write can blow past `LowLevelHooksTimeout` (300ms) and
// Windows will silently disable the hook.

unsafe extern "system" fn keyboard_proc(n_code: i32, w_param: usize, l_param: isize) -> isize {
    if n_code == HC_ACTION {
        let kb = &*(l_param as *const KBDLLHOOKSTRUCT);
        LL_HOOK_HITS.fetch_add(1, Ordering::Relaxed);

        // Alt+Space — swallow so it doesn't pop the system menu.
        if w_param == WM_SYSKEYDOWN as usize
            && kb.vk_code == VK_SPACE
            && (kb.flags & LLKHF_ALTDOWN) != 0
            && SUPPRESS.load(Ordering::Relaxed)
        {
            return 1;
        }

        // Escape — two paths:
        //   1. QS has foreground focus → passthrough to webview (JS handler).
        //   2. QS visible but not focused → PostMessage → subclass hides it.
        if kb.vk_code == VK_ESCAPE {
            let qs_hwnd = QS_HWND.load(Ordering::Relaxed);
            if qs_hwnd != 0 && IsWindowVisible(qs_hwnd) != 0 {
                LL_HOOK_ESC_HITS.fetch_add(1, Ordering::Relaxed);
                let fg = GetForegroundWindow();
                if fg == qs_hwnd {
                    LL_ESC_PASSTHROUGH.fetch_add(1, Ordering::Relaxed);
                    return CallNextHookEx(
                        HOOK_HANDLE.load(Ordering::Relaxed),
                        n_code,
                        w_param,
                        l_param,
                    );
                }
                LL_ESC_POSTMSG.fetch_add(1, Ordering::Relaxed);
                PostMessageW(qs_hwnd, QS_HIDE_MSG, 0, 0);
                return 1;
            }
            LL_ESC_SKIP.fetch_add(1, Ordering::Relaxed);
        }
    }
    CallNextHookEx(HOOK_HANDLE.load(Ordering::Relaxed), n_code, w_param, l_param)
}

// ---- Window subclass proc ------------------------------------------------
//
// Runs on the main thread — `eprintln!` is safe here.

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

    if (msg == QS_HIDE_MSG
        || (msg == WM_KEYDOWN
            && (w_param as u32) == VK_ESCAPE
            && hwnd == QS_HWND.load(Ordering::Relaxed)))
        && hwnd == QS_HWND.load(Ordering::Relaxed)
    {
        let qs = QS_HWND.load(Ordering::Relaxed);
        let via = if msg == QS_HIDE_MSG { "QS_HIDE_MSG" } else { "WM_KEYDOWN" };
        let vis_before = IsWindowVisible(hwnd);
        ShowWindow(hwnd, SW_HIDE);
        let vis_after = IsWindowVisible(hwnd);
        eprintln!(
            "[windows_hook] subclass hide via {via} hwnd=0x{hwnd:x} qs=0x{qs:x} visible {vis_before}→{vis_after}"
        );
        return 0;
    }

    let orig = GetPropW(hwnd, ORIG_PROP.as_ptr());
    if orig != 0 {
        CallWindowProcW(orig, hwnd, msg, w_param, l_param)
    } else {
        0
    }
}

// ---- Wide-string property key for stashing the original WNDPROC -----------

const ORIG_PROP: &[u16] = &[
    b'T' as u16, b'o' as u16, b'o' as u16, b'l' as u16,
    b'B' as u16, b'e' as u16, b'n' as u16, b'c' as u16,
    b'h' as u16, b'O' as u16, b'r' as u16, b'i' as u16,
    b'g' as u16, b'W' as u16, b'n' as u16, b'd' as u16,
    b'P' as u16, b'r' as u16, b'o' as u16, b'c' as u16,
    0,
];

// ---- Installation ---------------------------------------------------------

/// Install both the LL keyboard hook and a subclass on the main window.
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
        let hmod = GetModuleHandleW(std::ptr::null());
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, keyboard_proc as HOOKPROC, hmod, 0);
        if hook != 0 {
            HOOK_HANDLE.store(hook, Ordering::Relaxed);
            eprintln!("[windows_hook] WH_KEYBOARD_LL installed (handle=0x{hook:x})");
        } else {
            eprintln!("[windows_hook] WH_KEYBOARD_LL install FAILED");
        }
    }
}

/// Subclass an arbitrary HWND so its `WM_SYSCOMMAND` / `SC_KEYMENU`
/// (Alt+Space-triggered system menu) gets swallowed. Idempotent.
pub fn subclass(hwnd: isize) {
    if hwnd == 0 {
        return;
    }
    unsafe {
        if GetPropW(hwnd, ORIG_PROP.as_ptr()) != 0 {
            return;
        }
        let orig = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, subclass_proc as *const () as isize);
        if orig == 0 {
            eprintln!("[windows_hook] subclass FAILED (SetWindowLongPtrW→0) for HWND=0x{hwnd:x}");
            return;
        }
        if SetPropW(hwnd, ORIG_PROP.as_ptr(), orig) == 0 {
            eprintln!("[windows_hook] subclass FAILED (SetPropW→0) for HWND=0x{hwnd:x}");
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, orig);
            return;
        }
        eprintln!("[windows_hook] subclassed HWND=0x{hwnd:x}");
    }
}

/// Strip `WS_SYSMENU` from a window so Alt+Space stops opening the system
/// menu for it. Prefer [`subclass`].
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
