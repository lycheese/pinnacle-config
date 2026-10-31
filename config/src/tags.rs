//! Tags 1–9 on every output (AwesomeWM-style: a window can be on several tags,
//! and several tags can be shown at once).

use pinnacle_api::input::{self, Bind, Mod};
use pinnacle_api::output;
use pinnacle_api::tag;
use pinnacle_api::window;

use crate::settings::Settings;

pub fn setup(s: &Settings) {
    let tag_names = s.tag_names;
    let m = s.mod_key;

    output::for_each_output(move |output| {
        let mut tags = tag::add(output, tag_names);
        if let Some(first) = tags.next() {
            first.set_active(true);
        }
    });

    for name in tag_names {
        input::keybind(m, name)
            .on_press(move || {
                if let Some(tag) = tag::get(name) {
                    tag.switch_to();
                }
            })
            .group("Tag")
            .description(format!("Switch to tag {name}"));

        input::keybind(m | Mod::CTRL, name)
            .on_press(move || {
                if let Some(tag) = tag::get(name) {
                    tag.toggle_active();
                }
            })
            .group("Tag")
            .description(format!("Also show tag {name}"));

        input::keybind(m | Mod::SHIFT, name)
            .on_press(move || {
                if let Some(tag) = tag::get(name)
                    && let Some(win) = window::get_focused()
                {
                    win.move_to_tag(&tag);
                }
            })
            .group("Tag")
            .description(format!("Move window to tag {name}"));

        input::keybind(m | Mod::CTRL | Mod::SHIFT, name)
            .on_press(move || {
                if let Some(tag) = tag::get(name)
                    && let Some(win) = window::get_focused()
                {
                    win.toggle_tag(&tag);
                }
            })
            .group("Tag")
            .description(format!("Toggle tag {name} on window"));
    }
}
