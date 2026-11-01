//! Plugins: self-contained features layered on top of the base config.
//!
//! Contract for a plugin:
//! - one file (or folder) in this directory,
//! - a `pub fn setup(settings: &Settings)` that registers binds, rules and signals,
//! - no state shared with other plugins except through the Pinnacle API,
//! - a doc comment at the top saying what it does and which binds it adds.
//!
//! To disable a plugin, comment out its line in `setup` below.

mod autostart;
mod desktop_env;
mod focus_history;
mod refresh_rate;
mod screenshot;
mod super_grab;
mod window_rules;

use crate::settings::Settings;

pub fn setup(settings: &Settings) {
    // First: programs spawned by later plugins inherit this environment.
    desktop_env::setup(settings);
    refresh_rate::setup(settings);
    window_rules::setup(settings);
    focus_history::setup(settings);
    screenshot::setup(settings);
    autostart::setup(settings);
    // Last: its long section goes to the bottom of the keybind overlay.
    super_grab::setup(settings);
}
