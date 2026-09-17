//! macOS profile checkboxes remain open and persist during native menu tracking.
use crate::{err, CommandResult};
use objc2::{
    define_class, msg_send, rc::Retained, runtime::AnyObject, sel, DeclaredClass, MainThreadMarker,
    MainThreadOnly,
};
use objc2_app_kit::{
    NSButton, NSButtonType, NSControlStateValueOff, NSControlStateValueOn, NSView,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use tauri::AppHandle;

struct Selection {
    app: AppHandle,
    profile_id: String,
}

define_class!(
    #[unsafe(super(NSButton))]
    #[name = "CodexSwitchProfileCheckbox"]
    #[thread_kind = MainThreadOnly]
    #[ivars = Selection]
    struct ProfileCheckbox;

    impl ProfileCheckbox {
        #[unsafe(method(toggleProfile:))]
        fn toggle_profile(&self, _sender: Option<&AnyObject>) {
            let selection = self.ivars();
            if let Some(checked) = super::select_from_native_menu(&selection.app, &selection.profile_id) {
                self.setState(if checked { NSControlStateValueOn } else { NSControlStateValueOff });
            }
        }
    }
);

impl ProfileCheckbox {
    fn new(mtm: MainThreadMarker, app: AppHandle, profile_id: String) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(Selection { app, profile_id });
        // SAFETY: NSButton's designated initializer, then an action declared above
        // on this exact class. NSControl's target is weak; the menu view owns us.
        unsafe {
            let button: Retained<Self> = msg_send![super(this), initWithFrame: NSRect::ZERO];
            button.setButtonType(NSButtonType::Switch);
            button.setTarget(Some(&button));
            button.setAction(Some(sel!(toggleProfile:)));
            button
        }
    }
}

pub(super) fn sync(
    app: &AppHandle,
    tray: &tauri::tray::TrayIcon,
    identity: &[(String, String)],
) -> CommandResult<()> {
    let app = app.clone();
    let ids = identity
        .iter()
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    tray.with_inner_tray_icon(move |inner| {
        let mtm = MainThreadMarker::new().expect("tray updates run on the main thread");
        let Some(menu) = inner.ns_status_item().and_then(|item| item.menu(mtm)) else {
            return;
        };
        let Some(submenu) = menu.itemArray().iter().find_map(|item| item.submenu()) else {
            return;
        };
        for (item, id) in submenu.itemArray().iter().zip(ids) {
            if let Some(view) = item.view() {
                for child in view.subviews() {
                    if let Some(button) = child.downcast_ref::<ProfileCheckbox>() {
                        button.setState(item.state());
                        button.setEnabled(item.isEnabled());
                    }
                }
                continue;
            }
            let Some(profile_id) = id.strip_prefix(super::PROFILE_PREFIX) else {
                continue;
            };
            let button = ProfileCheckbox::new(mtm, app.clone(), profile_id.to_owned());
            button.setTitle(&item.title());
            button.setState(item.state());
            button.setEnabled(item.isEnabled());
            button.sizeToFit();
            let width = button.frame().size.width.max(190.0);
            let view = NSView::initWithFrame(
                mtm.alloc(),
                NSRect::new(NSPoint::ZERO, NSSize::new(width + 24.0, 32.0)),
            );
            button.setFrame(NSRect::new(
                NSPoint::new(12.0, 4.0),
                NSSize::new(width, 24.0),
            ));
            view.addSubview(&button);
            item.setView(Some(&view));
        }
    })
    .map_err(err)
}
