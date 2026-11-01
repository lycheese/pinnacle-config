//! Autostart: programs launched once per compositor session.
//!
//! `.once()` means a config reload does not start a second copy.
//! Programs that are not installed are skipped silently.
//!
//! Waybar uses `waybar/config.jsonc.in`, rendered with the settings on every config start
//! into `$XDG_RUNTIME_DIR/pinnacle-config/waybar.jsonc`. A running waybar does not reread it;
//! restart waybar (or log in again) after changing the template or `[waybar]` settings.

use std::path::{Path, PathBuf};

use pinnacle_api::process::Command;

use crate::settings::Settings;

const AUTOSTART: &[&[&str]] = &[
    &["mako"],
    // Polkit agent: Fedora (mate-polkit), then Pop!_OS/Ubuntu (policykit-1-gnome).
    // Only the installed one starts.
    &["/usr/libexec/polkit-mate-authentication-agent-1"],
    &["/usr/lib/policykit-1-gnome/polkit-gnome-authentication-agent-1"],
];

const CORETEMP: &str = "/sys/devices/platform/coretemp.0/hwmon";

pub fn setup(settings: &Settings) {
    match render_waybar_config(settings) {
        Ok(path) => {
            Command::new("waybar")
                .args(["-c", &path.to_string_lossy()])
                .once()
                .spawn();
        }
        Err(err) => eprintln!("autostart: waybar config not written: {err}"),
    }

    for cmd in AUTOSTART {
        Command::new(cmd[0]).args(&cmd[1..]).once().spawn();
    }
}

fn render_waybar_config(settings: &Settings) -> std::io::Result<PathBuf> {
    let template = settings::repo_dir().join("waybar/config.jsonc.in");
    let text = std::fs::read_to_string(&template)?;

    let hwmon = settings.file.str("waybar.temperature_hwmon", "");
    let hwmon = if hwmon.is_empty() && Path::new(CORETEMP).exists() {
        CORETEMP.to_string()
    } else {
        hwmon
    };
    let source = if hwmon.is_empty() {
        r#""thermal-zone": 0,"#.to_string()
    } else {
        format!(r#""hwmon-path-abs": "{hwmon}", "input-filename": "temp1_input","#)
    };

    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("pinnacle-config");
    std::fs::create_dir_all(&dir)?;
    let out = dir.join("waybar.jsonc");
    std::fs::write(&out, text.replace("@TEMPERATURE_SOURCE@", &source))?;
    Ok(out)
}
