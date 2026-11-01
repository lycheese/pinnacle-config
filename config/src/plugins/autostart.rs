//! Autostart: programs launched once per compositor session.
//!
//! `.once()` means a config reload does not start a second copy.
//! Programs that are not installed are skipped silently.

use pinnacle_api::process::Command;

use crate::settings::Settings;

const AUTOSTART: &[&[&str]] = &[
    // The repo's own waybar config (absolute path, fixed at build time).
    &[
        "waybar",
        "-c",
        concat!(env!("CARGO_MANIFEST_DIR"), "/../waybar/config.jsonc"),
    ],
    &["mako"],
    // Polkit agent: Fedora (mate-polkit), then Pop!_OS/Ubuntu (policykit-1-gnome).
    // Only the installed one starts.
    &["/usr/libexec/polkit-mate-authentication-agent-1"],
    &["/usr/lib/policykit-1-gnome/polkit-gnome-authentication-agent-1"],
];

pub fn setup(_settings: &Settings) {
    for cmd in AUTOSTART {
        Command::new(cmd[0]).args(&cmd[1..]).once().spawn();
    }
}
