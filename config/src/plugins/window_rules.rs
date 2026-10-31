//! Window rules: decide how new windows appear, matched by app ID.
//!
//! Find an app's ID with `pctl windows` while it is open.

use pinnacle_api::window::{self, WindowHandle};

use crate::settings::Settings;

/// Apps that should open floating instead of tiled.
const FLOATING_APPS: &[&str] = &[
    "pavucontrol",
    "org.pulseaudio.pavucontrol",
    "nm-connection-editor",
    "blueman-manager",
];

/// Apps pinned to a tag: (app ID, tag name).
const TAG_PINS: &[(&str, &str)] = &[
    // ("org.mozilla.firefox", "2"),
];

pub fn setup(_settings: &Settings) {
    window::add_window_rule(|win: WindowHandle| {
        #[cfg(feature = "snowcap")]
        {
            use pinnacle_api::snowcap::FocusBorder;
            win.set_decoration_mode(window::DecorationMode::ServerSide);
            let _ = FocusBorder::new_with_titlebar(&win).decorate();
        }

        let app_id = win.app_id();

        if FLOATING_APPS.contains(&app_id.as_str()) {
            win.set_floating(true);
        }

        if let Some((_, tag_name)) = TAG_PINS.iter().find(|(id, _)| *id == app_id)
            && let Some(tag) = pinnacle_api::tag::get(*tag_name)
        {
            win.move_to_tag(&tag);
        }
    });

    // Rules only apply to new windows; decorate the ones that survived a config reload.
    #[cfg(feature = "snowcap")]
    for win in window::get_all() {
        let _ = pinnacle_api::snowcap::FocusBorder::new_with_titlebar(&win).decorate();
    }
}
