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

/// The quick-capture Helix window (scripts/capture/edit.sh): floating, centred, this size.
const CAPTURE_APP: &str = "pinnacle-capture";
const CAPTURE_SIZE: (u32, u32) = (720, 360);

/// Centres a window of size `(w, h)` on the focused output.
fn center(win: &WindowHandle, (w, h): (u32, u32)) {
    let Some(out) = pinnacle_api::output::get_focused() else {
        return;
    };
    let (Some(loc), Some(size)) = (out.loc(), out.logical_size()) else {
        return;
    };
    let x = loc.x + (size.w as i32 - w as i32) / 2;
    let y = loc.y + (size.h as i32 - h as i32) / 2;
    win.set_geometry(x, y, w, h);
}

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

        if app_id == CAPTURE_APP {
            win.set_floating(true);
            center(&win, CAPTURE_SIZE);
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
