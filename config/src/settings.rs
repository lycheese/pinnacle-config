//! Values you will want to change first.

use pinnacle_api::input::Mod;
use pinnacle_api::pinnacle::{self, Backend};

pub struct Settings {
    /// Main modifier. `Super` on a real session, `Alt` when running nested inside GNOME,
    /// so the host desktop keeps its own Super shortcuts.
    pub mod_key: Mod,
    pub terminal: &'static str,
    pub launcher: &'static [&'static str],
    /// Gap between windows and around the screen edge, in logical pixels.
    pub gaps: f32,
    pub tag_names: [&'static str; 9],
}

impl Settings {
    pub fn detect() -> Self {
        let mod_key = match pinnacle::backend() {
            Backend::Tty => Mod::SUPER,
            Backend::Window => Mod::ALT,
        };

        Self {
            mod_key,
            terminal: "alacritty",
            launcher: &["fuzzel"],
            gaps: 6.0,
            tag_names: ["1", "2", "3", "4", "5", "6", "7", "8", "9"],
        }
    }
}
