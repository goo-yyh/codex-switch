use tauri::{
    menu::{Menu, PredefinedMenuItem, Submenu},
    AppHandle, Manager, Wry,
};

/// Editing actions are shortcut-only; configure_visibility removes their visible rows
/// after Tauri installs the native menu, without breaking input keyboard shortcuts.
pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let locale = app
        .try_state::<crate::state::AppState>()
        .and_then(|state| {
            state
                .store
                .lock()
                .ok()
                .and_then(|store| crate::locale::Locale::read(&store).ok())
        })
        .unwrap_or_default();
    let application = Submenu::with_items(
        app,
        "Codex Switch",
        true,
        &[
            &PredefinedMenuItem::undo(app, Some(locale.text("撤销", "Undo")))?,
            &PredefinedMenuItem::redo(app, Some(locale.text("重做", "Redo")))?,
            &PredefinedMenuItem::cut(app, Some(locale.text("剪切", "Cut")))?,
            &PredefinedMenuItem::copy(app, Some(locale.text("复制", "Copy")))?,
            &PredefinedMenuItem::paste(app, Some(locale.text("粘贴", "Paste")))?,
            &PredefinedMenuItem::select_all(app, Some(locale.text("全选", "Select All")))?,
            &PredefinedMenuItem::hide(
                app,
                Some(locale.text("隐藏 Codex Switch", "Hide Codex Switch")),
            )?,
            &PredefinedMenuItem::hide_others(
                app,
                Some(locale.text("隐藏其他应用", "Hide Others")),
            )?,
            &PredefinedMenuItem::separator(app)?,
            // Quit still goes through the app's existing ExitRequested guard.
            &PredefinedMenuItem::quit(app, Some(locale.text("退出应用", "Quit App")))?,
        ],
    )?;
    Menu::with_items(app, &[&application])
}

pub fn configure_visibility() -> Result<(), &'static str> {
    use objc2::{sel, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSMenu, NSUserInterfaceItemIdentification};

    fn hide_in(menu: &NSMenu) {
        for item in menu.itemArray() {
            // AppKit appends these to menus containing editing commands, even
            // when those commands are hidden. Match native identity, not the
            // localized title (AutoFill / 自动填充).
            let system_input_item = item.action().is_some_and(|action| {
                [sel!(startDictation:), sel!(orderFrontCharacterPalette:)].contains(&action)
            }) || item.identifier().is_some_and(|identifier| {
                identifier.to_string() == "_NSMenuItemAutoFillIdentifier"
            });
            if system_input_item {
                menu.removeItem(&item);
                continue;
            }
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
        // Remove the separator AppKit leaves after the final visible command.
        // Hidden shortcut entries do not count as visible content.
        let items = menu.itemArray();
        for index in (0..items.len()).rev() {
            let item = items.objectAtIndex(index);
            if item.isHidden() {
                continue;
            }
            if !item.isSeparatorItem() {
                break;
            }
            menu.removeItem(&item);
        }
    }

    let main_thread = MainThreadMarker::new().ok_or("菜单必须在主线程初始化")?;
    let menu = NSApplication::sharedApplication(main_thread)
        .mainMenu()
        .ok_or("应用菜单尚未初始化")?;
    hide_in(&menu);
    Ok(())
}
