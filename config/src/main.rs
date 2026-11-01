//! Personal Pinnacle config.
//!
//! The config is a normal program that talks to the compositor over gRPC. Pinnacle starts it via
//! `pinnacle.toml`, and restarts it on every reload, so changes here never require restarting the
//! compositor itself.
//!
//! Layout of this crate:
//! - `settings.rs`  – values you will want to tweak first (mod key, apps, gaps)
//! - `binds.rs`     – compositor, window and app keybinds
//! - `layouts.rs`   – dynamic tiling layouts and the layout-cycling binds
//! - `tags.rs`      – tags 1–9 per output and their binds
//! - `plugins/`     – self-contained features; each one is a module with a `setup` function
//! - `leader/`      – the Mod+Space which-key menu (needs Snowcap)

mod binds;
mod layouts;
#[cfg(feature = "snowcap")]
mod leader;
mod plugins;
mod settings;
mod tags;

use pinnacle_api::output;
use pinnacle_api::signal::{OutputSignal, WindowSignal};
use pinnacle_api::window;

use crate::settings::Settings;

async fn config() {
    let settings = Settings::detect();

    // A reload does not reset the active bind layer. If the leader menu's layer was active when
    // the config restarted, the normal binds would stay dead; always start in the default layer.
    pinnacle_api::input::BindLayer::DEFAULT.enter();

    binds::setup(&settings);
    layouts::setup(&settings);
    tags::setup(&settings);
    plugins::setup(&settings);
    #[cfg(feature = "snowcap")]
    leader::setup(&settings);

    // Sloppy focus: focus follows the pointer between windows and outputs.
    window::connect_signal(WindowSignal::PointerEnter(Box::new(|win| {
        win.set_focused(true);
    })));
    output::connect_signal(OutputSignal::PointerEnter(Box::new(|output| {
        output.focus();
    })));

    #[cfg(feature = "snowcap")]
    if let Some(error) = pinnacle_api::pinnacle::take_last_error() {
        // The previous config run crashed (e.g. a panic after a reload): show why.
        pinnacle_api::snowcap::ConfigCrashedMessage::new(error).show();
    }
}

pinnacle_api::main!(config);
