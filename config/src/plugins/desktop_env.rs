//! Desktop environment name: lets Electron/Chromium apps use the GNOME keyring.
//!
//! As a session, Pinnacle sets `XDG_CURRENT_DESKTOP=pinnacle`. Electron and Chromium do not
//! recognise that name and fall back to storing secrets unencrypted, or not at all (the Claude
//! desktop app then forgets its sign-in). Adding `GNOME` as a second entry makes them use the
//! Secret Service (gnome-keyring). `pinnacle` stays first, so xdg-desktop-portal still reads
//! `pinnacle-portals.conf`.
//!
//! Only programs started by the compositor get the new value. The systemd and D-Bus activation
//! environments keep `pinnacle`, so GNOME-only autostart entries do not start.
//!
//! No binds. Must run before any plugin that spawns programs.

use pinnacle_api::process;

use crate::settings::Settings;

const DESKTOP: &str = "pinnacle:GNOME";

pub fn setup(_settings: &Settings) {
    // Nested runs keep the host desktop's value; only change Pinnacle's own session value.
    if std::env::var("XDG_CURRENT_DESKTOP").as_deref() == Ok("pinnacle") {
        process::set_env("XDG_CURRENT_DESKTOP", DESKTOP);
    }
}
