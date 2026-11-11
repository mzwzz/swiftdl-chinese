//! Small per-download windows, like IDM's: "New download" when the browser
//! hands over a file, then live progress, then "Download complete".

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// Frameless windows on Windows 11 get a 1px light border from the system,
/// which looks like a white frame in dark mode. Ask for no border and
/// rounded corners; the shadow stays.
pub fn polish(window: &WebviewWindow) {
    #[cfg(windows)]
    {
        use std::ffi::c_void;
        #[link(name = "dwmapi")]
        unsafe extern "system" {
            fn DwmSetWindowAttribute(hwnd: *mut c_void, attr: u32, value: *const c_void, size: u32) -> i32;
        }
        const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
        const DWMWA_BORDER_COLOR: u32 = 34;
        const DWMWCP_ROUND: u32 = 2;
        const DWMWA_COLOR_NONE: u32 = 0xFFFF_FFFE;
        if let Ok(hwnd) = window.hwnd() {
            unsafe {
                DwmSetWindowAttribute(hwnd.0, DWMWA_WINDOW_CORNER_PREFERENCE, (&DWMWCP_ROUND as *const u32).cast(), 4);
                DwmSetWindowAttribute(hwnd.0, DWMWA_BORDER_COLOR, (&DWMWA_COLOR_NONE as *const u32).cast(), 4);
            }
        }
    }
    #[cfg(not(windows))]
    let _ = window;
}

/// Card width plus the transparent margin the page uses for its shadow.
const WINDOW_WIDTH: f64 = 520.0 + 2.0 * 16.0;

/// macOS: activate the app itself, like a Dock click or `open` would. The
/// app is usually a background process when the browser hands over a
/// download, and macOS won't raise or focus a background process's windows
/// (nor hand them the keyboard) until the app is active. Uses the modern
/// `NSRunningApplication` API, since `NSApp activateIgnoringOtherApps:` is
/// deprecated. Must run on the main thread.
#[cfg(target_os = "macos")]
pub fn activate_app() {
    use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};
    unsafe {
        let _ = NSRunningApplication::currentApplication()
            .activateWithOptions(NSApplicationActivationOptions::ActivateIgnoringOtherApps);
    }
}

/// macOS: make a window visible on every Space, including over fullscreen
/// apps. A normal window — even a floating one — belongs to the Space it
/// opened on, so while the browser runs fullscreen the download dialog
/// opens in another Space and stays invisible: exactly "not coming to the
/// front". CanJoinAllSpaces + FullScreenAuxiliary lets the dialog follow
/// the user into the fullscreen Space. Must run on the main thread.
#[cfg(target_os = "macos")]
fn float_over_spaces(w: &WebviewWindow) {
    use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};
    let handle = w.clone();
    let inner = w.clone();
    let _ = handle.run_on_main_thread(move || {
        let Ok(ns) = inner.ns_window() else {
            crate::log::line("could not float over Spaces: no ns_window yet");
            return;
        };
        unsafe {
            let win: &NSWindow = &*(ns.cast::<NSWindow>());
            win.setCollectionBehavior(
                NSWindowCollectionBehavior::CanJoinAllSpaces | NSWindowCollectionBehavior::FullScreenAuxiliary,
            );
            // A plain floating level (~3-5) is not admitted into another
            // app's fullscreen Space on modern macOS; modal-panel level (8,
            // where helpers like translation popovers live) is. Re-asserted
            // periodically, because the window toolkit re-applies its own
            // always-on-top level asynchronously and would win the race.
            win.setLevel(8); // kCGModalPanelWindowLevel
        }
    });
}

/// Puts a window in front of whatever the user is doing, like IDM's dialogs.
/// Windows only lets a background app flash its taskbar button, but a
/// window that is briefly "always on top" still appears above everything;
/// it then drops back to a normal window, staying in front.
fn bring_to_front(w: &WebviewWindow) {
    let _ = w.unminimize();
    // macOS: activate the app before ordering the window front. The window
    // can be shown above the browser either way, but only an active app
    // takes the keyboard and pulls its Space to the front.
    #[cfg(target_os = "macos")]
    {
        let _ = w.run_on_main_thread(activate_app);
    }
    let _ = w.show();
    let _ = w.set_always_on_top(true);
    #[cfg(target_os = "macos")]
    float_over_spaces(w);
    let _ = w.set_focus();
    let w = w.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(600));
        let _ = w.set_focus();
        #[cfg(target_os = "macos")]
        {
            // These small dialogs stay floating over every Space (including
            // fullscreen apps) for their whole life, like IDM's windows.
            // Re-assert the Space behavior and level periodically: the
            // window toolkit re-applies its own always-on-top level async
            // and would otherwise win the race against the first assert.
            // Focus stays best-effort: keep asking until the window is key
            // (activation granted, or the user clicked it), then stop.
            loop {
                if !w.is_visible().unwrap_or(false) {
                    return;
                }
                float_over_spaces(&w);
                if !w.is_focused().unwrap_or(false) {
                    let _ = w.run_on_main_thread(activate_app);
                }
                std::thread::sleep(std::time::Duration::from_millis(800));
            }
        }
        #[cfg(not(target_os = "macos"))]
        let _ = w.set_always_on_top(false);
    });
}

/// Opens `download.html?{query}` in a window called `label`, or brings an
/// existing one forward.
///
/// The window is built on its own thread. Building one on the main thread,
/// which is where synchronous commands run, deadlocks WebView2 on Windows:
/// pasting a YouTube link froze the whole app (#4).
pub fn open(app: &AppHandle, label: &str, query: &str, title: &str) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window(label) {
        bring_to_front(&w);
        return Ok(());
    }
    let (app, label, query, title) = (app.clone(), label.to_string(), query.to_string(), title.to_string());
    std::thread::spawn(move || {
        // Transparent and frameless: the page draws the whole window (rounded
        // card and shadow), so no native frame, border or unpainted edge shows.
        let built = WebviewWindowBuilder::new(&app, &label, WebviewUrl::App(format!("download.html?{query}").into()))
            .title(&title)
            .inner_size(WINDOW_WIDTH, 360.0)
            .resizable(false)
            .maximizable(false)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .center()
            .focused(true)
            .always_on_top(true)
            .build();
        match built {
            Ok(w) => bring_to_front(&w),
            Err(e) => crate::log::line(&format!("could not open window {label}: {e:#}")),
        }
    });
    Ok(())
}
