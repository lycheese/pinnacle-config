//! Super grab: every Super+key combo goes to the compositor, never to the focused app.
//!
//! Unbound combos are swallowed, so apps do not react to Super shortcuts. This works by
//! adding a do-nothing bind for every key with Super held and Shift, Ctrl, Alt and the Neo level
//! modifiers ignored. Real binds still run: Pinnacle fires every bind that matches a key.
//!
//! Pinnacle matches a key by its base (level 1) symbol, so one bind per physical key is enough.
//! Apps that inhibit shortcuts (VMs, remote desktops) still get Super combos.
//!
//! The keybind overlay lists every grabbed key in a "Super grab" section. The API cannot hide
//! binds from the overlay, so this plugin is registered last to keep that section at the bottom.

use pinnacle_api::input::{self, Bind, Keysym, Mod};

use crate::settings::Settings;

/// Super held, everything else ignored.
const MODS: Mod = Mod::SUPER
    .union(Mod::IGNORE_SHIFT)
    .union(Mod::IGNORE_CTRL)
    .union(Mod::IGNORE_ALT)
    .union(Mod::IGNORE_ISO_LEVEL3_SHIFT)
    .union(Mod::IGNORE_ISO_LEVEL5_SHIFT);

pub fn setup(_settings: &Settings) {
    for key in keys() {
        input::keybind(MODS, key)
            .on_press(|| {})
            .group("Super grab")
            .description("Swallowed, not sent to apps");
    }
}

/// Base symbols of every key worth grabbing (US, German and Neo layouts).
fn keys() -> impl Iterator<Item = Keysym> {
    // Latin-1 keysyms equal their code points: space, digits, punctuation, lowercase letters.
    let ascii = (0x20..=0x7e_u32).filter(|c| !(0x41..=0x5a).contains(c));
    let german = [0xe4, 0xf6, 0xfc, 0xdf]; // ä ö ü ß
    let dead_keys = 0xfe50..=0xfe5a; // dead_grave .. dead_ogonek, level 1 on Neo
    let function_keys = 0xffbe..=0xffd5; // F1 .. F24
    let editing = [
        0xff08, 0xff09, 0xff0d, 0xff1b, 0xffff, // BackSpace Tab Return Escape Delete
        0xff50, 0xff51, 0xff52, 0xff53, 0xff54, // Home Left Up Right Down
        0xff55, 0xff56, 0xff57, // Prior Next End
        0xff61, 0xff63, 0xff67, // Print Insert Menu
    ];

    ascii
        .chain(german)
        .chain(dead_keys)
        .chain(function_keys)
        .chain(editing)
        .map(Keysym::from)
}
