//! Screenshots with grim and slurp.
//!
//! Mod+S: drag to select a region, or click a monitor to take all of it. Esc cancels.
//! The image is saved to `~/Pictures/Screenshots/` (your XDG pictures folder), copied to the
//! clipboard, and announced with a notification.
//!
//! Needs grim, slurp, wl-clipboard and notify-send (installed by the install scripts).

use pinnacle_api::input::{self, Bind};
use pinnacle_api::process::Command;

use crate::settings::Settings;

/// `slurp` exits non-zero on Esc, which ends the script before grim runs.
const SCRIPT: &str = r#"
dir="$(xdg-user-dir PICTURES 2>/dev/null || echo "$HOME/Pictures")/Screenshots"
mkdir -p "$dir"
file="$dir/$(date +%Y-%m-%d_%H-%M-%S).png"
geometry="$(slurp -o)" || exit 0
grim -g "$geometry" "$file" || exit 1
wl-copy --type image/png < "$file"
notify-send --icon "$file" "Screenshot saved" "${file#"$HOME"/}, copied to clipboard"
"#;

pub fn setup(settings: &Settings) {
    input::keybind(settings.mod_key, 's')
        .on_press(|| {
            Command::new("sh").args(["-c", SCRIPT]).spawn();
        })
        .group("Apps")
        .description("Screenshot: drag a region or click a monitor");
}
