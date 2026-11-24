//! macOS menu bar, in Chinese.
//!
//! Tauri ships an English default menu (File / Edit / View / Window / Help and
//! the items under them). The whole app is localized, so the menu has to be
//! too: this builds the same menu again with Chinese labels, keeping the
//! predefined items where they carry native behaviour (copy/paste, hide, quit,
//! close window…) so the standard shortcuts keep working, and wiring the few
//! app-specific entries (新建下载 / 打开下载文件夹 / 打开日志) to our code.

use std::sync::Arc;

use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Manager, Wry};

use crate::manager::DownloadManager;

pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    // ── SwiftDL（应用菜单） ──
    let about = PredefinedMenuItem::about(
        app,
        Some("关于 SwiftDL"),
        Some(AboutMetadata {
            name: Some("SwiftDL".into()),
            version: Some(env!("CARGO_PKG_VERSION").into()),
            copyright: Some("MIT 协议 · 基于 harshsinghsv/swiftdl 汉化".into()),
            credits: Some("多连接下载 · 断点续传 · 浏览器接管 · 视频与种子".into()),
            ..Default::default()
        }),
    )?;
    let services = PredefinedMenuItem::services(app, Some("服务"))?;
    let hide = PredefinedMenuItem::hide(app, Some("隐藏 SwiftDL"))?;
    let hide_others = PredefinedMenuItem::hide_others(app, Some("隐藏其他"))?;
    let show_all = PredefinedMenuItem::show_all(app, Some("全部显示"))?;
    let quit = PredefinedMenuItem::quit(app, Some("退出 SwiftDL"))?;
    let sep_a = PredefinedMenuItem::separator(app)?;
    let sep_b = PredefinedMenuItem::separator(app)?;
    let sep_c = PredefinedMenuItem::separator(app)?;
    let app_menu = Submenu::with_items(
        app,
        "SwiftDL",
        true,
        &[&about, &sep_a, &services, &sep_b, &hide, &hide_others, &show_all, &sep_c, &quit],
    )?;

    // ── 文件 ──
    let new_download = MenuItem::with_id(app, "new-download", "新建下载…", true, Some("CmdOrCtrl+N"))?;
    let open_downloads = MenuItem::with_id(app, "open-downloads", "打开下载文件夹", true, None::<&str>)?;
    let open_log = MenuItem::with_id(app, "open-log", "打开日志文件", true, None::<&str>)?;
    let close_window = PredefinedMenuItem::close_window(app, Some("关闭窗口"))?;
    let sep_file = PredefinedMenuItem::separator(app)?;
    let file_menu = Submenu::with_items(
        app,
        "文件",
        true,
        &[&new_download, &open_downloads, &open_log, &sep_file, &close_window],
    )?;

    // ── 编辑 ──
    let undo = PredefinedMenuItem::undo(app, Some("撤销"))?;
    let redo = PredefinedMenuItem::redo(app, Some("重做"))?;
    let cut = PredefinedMenuItem::cut(app, Some("剪切"))?;
    let copy = PredefinedMenuItem::copy(app, Some("拷贝"))?;
    let paste = PredefinedMenuItem::paste(app, Some("粘贴"))?;
    let select_all = PredefinedMenuItem::select_all(app, Some("全选"))?;
    let sep_edit_1 = PredefinedMenuItem::separator(app)?;
    let edit_menu = Submenu::with_items(
        app,
        "编辑",
        true,
        &[&undo, &redo, &sep_edit_1, &cut, &copy, &paste, &select_all],
    )?;

    // ── 显示 ──
    let fullscreen = PredefinedMenuItem::fullscreen(app, Some("进入全屏"))?;
    let view_menu = Submenu::with_items(app, "显示", true, &[&fullscreen])?;

    // ── 窗口 ──
    let minimize = PredefinedMenuItem::minimize(app, Some("最小化"))?;
    let zoom = PredefinedMenuItem::maximize(app, Some("缩放"))?;
    let bring_all = PredefinedMenuItem::bring_all_to_front(app, Some("前置全部窗口"))?;
    let sep_win = PredefinedMenuItem::separator(app)?;
    let window_menu = Submenu::with_items(app, "窗口", true, &[&minimize, &zoom, &sep_win, &bring_all])?;

    // ── 帮助 ──
    let project = MenuItem::with_id(app, "help-project", "SwiftDL 项目主页", true, None::<&str>)?;
    let help_menu = Submenu::with_items(app, "帮助", true, &[&project])?;

    Menu::with_items(app, &[&app_menu, &file_menu, &edit_menu, &view_menu, &window_menu, &help_menu])
}

/// Clicks from the menu bar. Returns true when the event was one of ours.
pub fn handle(app: &AppHandle, id: &str) -> bool {
    use tauri_plugin_opener::OpenerExt;
    match id {
        "new-download" => {
            crate::show_main_window(app);
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.eval("window.dispatchEvent(new Event('swiftdl:new-download'))");
            }
        }
        "open-downloads" => {
            let dir = app.state::<Arc<DownloadManager>>().settings().download_dir;
            let _ = app.opener().open_path(dir.to_string_lossy(), None::<&str>);
        }
        "open-log" => {
            let p = crate::log::path();
            if !p.exists() {
                crate::log::line("log opened");
            }
            let _ = app.opener().open_path(p.to_string_lossy(), None::<&str>);
        }
        "help-project" => {
            let _ = app.opener().open_url("https://github.com/harshsinghsv/swiftdl", None::<&str>);
        }
        _ => return false,
    }
    true
}
