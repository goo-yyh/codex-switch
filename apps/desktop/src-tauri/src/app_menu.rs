use tauri::{
    menu::{Menu, PredefinedMenuItem, Submenu},
    AppHandle, Wry,
};

/// Editing actions are shortcut-only; hide_edit_actions removes their visible rows
/// after Tauri installs the native menu, without breaking input keyboard shortcuts.
pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let application = Submenu::with_items(
        app,
        "Codex Switch",
        true,
        &[
            &PredefinedMenuItem::undo(app, Some("撤销"))?,
            &PredefinedMenuItem::redo(app, Some("重做"))?,
            &PredefinedMenuItem::cut(app, Some("剪切"))?,
            &PredefinedMenuItem::copy(app, Some("复制"))?,
            &PredefinedMenuItem::paste(app, Some("粘贴"))?,
            &PredefinedMenuItem::select_all(app, Some("全选"))?,
            &PredefinedMenuItem::hide(app, Some("隐藏 Codex Switch"))?,
            &PredefinedMenuItem::hide_others(app, Some("隐藏其他应用"))?,
            &PredefinedMenuItem::separator(app)?,
            // Quit still goes through the app's existing ExitRequested guard.
            &PredefinedMenuItem::quit(app, Some("退出 Codex Switch"))?,
        ],
    )?;
    Menu::with_items(app, &[&application])
}

pub fn hide_edit_actions() -> Result<(), &'static str> {
    use objc2::{sel, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSMenu};

    fn hide_in(menu: &NSMenu) {
        for item in menu.itemArray() {
            if item.action().is_some_and(|action| {
                [
                    sel!(undo:),
                    sel!(redo:),
                    sel!(cut:),
                    sel!(copy:),
                    sel!(paste:),
                    sel!(selectAll:),
                ]
                .contains(&action)
            }) {
                item.setAllowsKeyEquivalentWhenHidden(true);
                item.setHidden(true);
            }
            if let Some(submenu) = item.submenu() {
                hide_in(&submenu);
            }
        }
    }

    let main_thread = MainThreadMarker::new().ok_or("菜单必须在主线程初始化")?;
    let menu = NSApplication::sharedApplication(main_thread)
        .mainMenu()
        .ok_or("应用菜单尚未初始化")?;
    hide_in(&menu);
    Ok(())
}
