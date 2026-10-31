//! Autostart: programs launched once per compositor session.
//!
//! `.once()` means a config reload does not start a second copy.
//! Programs that are not installed are skipped silently.

use pinnacle_api::process::Command;

use crate::settings::Settings;

const AUTOSTART: &[&[&str]] = &[
    &["waybar"],
    &["mako"],
    &["/usr/libexec/polkit-gnome-authentication-agent-1"],
];

pub fn setup(_settings: &Settings) {
    for cmd in AUTOSTART {
        Command::new(cmd[0]).args(&cmd[1..]).once().spawn();
    }
}
