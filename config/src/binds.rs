//! Keybinds and mousebinds that are not tied to layouts or tags.
//!
//! Every bind gets a `group` and `description`; they show up in the keybind overlay (Mod+S)
//! and keep the config self-documenting.

use pinnacle_api::input::{self, Bind, Keysym, Mod, MouseButton};
use pinnacle_api::process::Command;
use pinnacle_api::util::Direction;
use pinnacle_api::window;

use crate::settings::Settings;

pub fn setup(s: &Settings) {
    compositor(s);
    windows(s);
    apps(s);
    media();
}

/// Never remove the quit and reload binds: they are your way out if a config change misbehaves.
fn compositor(s: &Settings) {
    let m = s.mod_key;

    #[cfg(feature = "snowcap")]
    {
        input::keybind(m, 's')
            .on_press(|| {
                pinnacle_api::snowcap::BindOverlay::new().show();
            })
            .group("Compositor")
            .description("Show the keybind overlay");

        input::keybind(m | Mod::SHIFT, 'q')
            .on_press(|| {
                pinnacle_api::snowcap::QuitPrompt::new().show();
            })
            .group("Compositor")
            .description("Show quit prompt");

        input::keybind(m | Mod::CTRL | Mod::SHIFT, 'q')
            .set_as_quit()
            .group("Compositor")
            .description("Quit Pinnacle without prompt");
    }

    #[cfg(not(feature = "snowcap"))]
    input::keybind(m | Mod::SHIFT, 'q')
        .set_as_quit()
        .group("Compositor")
        .description("Quit Pinnacle");

    input::keybind(m | Mod::CTRL, 'r')
        .set_as_reload_config()
        .group("Compositor")
        .description("Reload the config");
}

fn windows(s: &Settings) {
    let m = s.mod_key;

    input::mousebind(m, MouseButton::Left)
        .on_press(|| {
            window::begin_move(MouseButton::Left);
        })
        .group("Mouse")
        .description("Move window");

    input::mousebind(m, MouseButton::Right)
        .on_press(|| {
            window::begin_resize(MouseButton::Right);
        })
        .group("Mouse")
        .description("Resize window");

    input::keybind(m | Mod::SHIFT, 'c')
        .on_press(|| {
            if let Some(win) = window::get_focused() {
                win.close();
            }
        })
        .group("Window")
        .description("Close the focused window");

    input::keybind(m | Mod::CTRL, Keysym::space)
        .on_press(|| {
            if let Some(win) = window::get_focused() {
                win.toggle_floating();
                win.raise();
            }
        })
        .group("Window")
        .description("Toggle floating");

    input::keybind(m, 'f')
        .on_press(|| {
            if let Some(win) = window::get_focused() {
                win.toggle_fullscreen();
                win.raise();
            }
        })
        .group("Window")
        .description("Toggle fullscreen");

    input::keybind(m, 'm')
        .on_press(|| {
            if let Some(win) = window::get_focused() {
                win.toggle_maximized();
                win.raise();
            }
        })
        .group("Window")
        .description("Toggle maximized");

    // Arrow keys: the same on every layout (Neo Bone, Miryoku's nav layer).
    let directions = [
        (Keysym::Left, Direction::Left, "left"),
        (Keysym::Down, Direction::Down, "down"),
        (Keysym::Up, Direction::Up, "up"),
        (Keysym::Right, Direction::Right, "right"),
    ];

    for (key, dir, name) in directions {
        input::keybind(m, key)
            .on_press(move || {
                if let Some(target) =
                    window::get_focused().and_then(|win| win.in_direction(dir).next())
                {
                    target.set_focused(true);
                }
            })
            .group("Window")
            .description(format!("Focus the window {name}"));

        input::keybind(m | Mod::SHIFT, key)
            .on_press(move || {
                if let Some(win) = window::get_focused()
                    && let Some(target) = win.in_direction(dir).next()
                {
                    win.swap(&target);
                }
            })
            .group("Window")
            .description(format!("Swap with the window {name}"));
    }
}

fn apps(s: &Settings) {
    let m = s.mod_key;
    let terminal = s.terminal;
    let launcher = s.launcher;

    input::keybind(m, Keysym::Return)
        .on_press(move || {
            Command::new(terminal).spawn();
        })
        .group("Apps")
        .description("Open a terminal");

    input::keybind(m, 'r')
        .on_press(move || {
            Command::new(launcher[0]).args(&launcher[1..]).spawn();
        })
        .group("Apps")
        .description("Open the app launcher");
}

fn media() {
    let wpctl = |args: &'static [&'static str]| {
        move || {
            Command::new("wpctl").args(args).spawn();
        }
    };

    let binds: [(Keysym, &'static [&'static str], &str); 4] = [
        (
            Keysym::XF86_AudioRaiseVolume,
            &["set-volume", "@DEFAULT_AUDIO_SINK@", "0.02+", "-l", "1.0"],
            "Volume up",
        ),
        (
            Keysym::XF86_AudioLowerVolume,
            &["set-volume", "@DEFAULT_AUDIO_SINK@", "0.02-"],
            "Volume down",
        ),
        (
            Keysym::XF86_AudioMute,
            &["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"],
            "Toggle mute",
        ),
        (
            Keysym::XF86_AudioMicMute,
            &["set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"],
            "Toggle mic mute",
        ),
    ];

    for (key, args, desc) in binds {
        input::keybind(Mod::empty(), key)
            .on_press(wpctl(args))
            .allow_when_locked()
            .group("Media")
            .description(desc);
    }

    let brightness = [
        (Keysym::XF86_MonBrightnessUp, "+10%", "Brightness up"),
        (Keysym::XF86_MonBrightnessDown, "10%-", "Brightness down"),
    ];

    for (key, step, desc) in brightness {
        input::keybind(Mod::empty(), key)
            .on_press(move || {
                Command::new("brightnessctl")
                    .args(["--class=backlight", "set", step])
                    .spawn();
            })
            .allow_when_locked()
            .group("Display")
            .description(desc);
    }
}
