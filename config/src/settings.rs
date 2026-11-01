//! Runtime settings: the mod key plus everything from `settings.toml` / `local.toml`.

use pinnacle_api::input::Mod;
use pinnacle_api::pinnacle::{self, Backend};
use pinnacle_api::process::Command;

pub struct Settings {
    /// Main modifier. `Super` on a real session, `Alt` when running nested inside GNOME,
    /// so the host desktop keeps its own Super shortcuts.
    pub mod_key: Mod,
    /// Shell command lines from `[apps]`.
    pub terminal: String,
    pub launcher: String,
    /// Gap between windows and around the screen edge, in logical pixels.
    pub gaps: f32,
    pub tag_names: [&'static str; 9],
    /// The merged settings files, for lookups by dotted path.
    pub file: settings::Settings,
}

impl Settings {
    pub fn detect() -> Self {
        let mod_key = match pinnacle::backend() {
            Backend::Tty => Mod::SUPER,
            Backend::Window => Mod::ALT,
        };

        let file = settings::Settings::load();
        Self {
            mod_key,
            terminal: file.str("apps.terminal", "alacritty"),
            launcher: file.str("apps.launcher", "fuzzel"),
            gaps: 6.0,
            tag_names: ["1", "2", "3", "4", "5", "6", "7", "8", "9"],
            file,
        }
    }
}

#[cfg(test)]
impl Settings {
    /// The repo's `settings.toml` defaults, without `local.toml` or a running compositor.
    pub fn for_test() -> Self {
        let file = settings::Settings::parse(include_str!("../../settings.toml"))
            .expect("settings.toml parses");
        Self {
            mod_key: Mod::SUPER,
            terminal: "alacritty".into(),
            launcher: "fuzzel".into(),
            gaps: 6.0,
            tag_names: ["1", "2", "3", "4", "5", "6", "7", "8", "9"],
            file,
        }
    }
}

/// Runs a command line with `sh -c`, so settings may contain arguments.
pub fn sh(command_line: &str) {
    Command::new("sh").args(["-c", command_line]).spawn();
}
