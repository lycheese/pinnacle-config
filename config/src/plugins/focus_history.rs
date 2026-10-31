//! Focus history: Mod+Tab jumps back to the previously focused window,
//! switching to its tag if necessary (like Alt+Tab between two windows).

use std::sync::{Arc, Mutex};

use pinnacle_api::input::{self, Bind, Keysym};
use pinnacle_api::signal::WindowSignal;
use pinnacle_api::window::{self, WindowHandle};

use crate::settings::Settings;

#[derive(Default)]
struct History {
    current: Option<u32>,
    previous: Option<u32>,
}

pub fn setup(s: &Settings) {
    let history = Arc::new(Mutex::new(History::default()));

    window::connect_signal(WindowSignal::Focused(Box::new({
        let history = history.clone();
        move |win| {
            let mut h = history.lock().unwrap();
            let id = win.id();
            if h.current != Some(id) {
                h.previous = h.current;
                h.current = Some(id);
            }
        }
    })));

    input::keybind(s.mod_key, Keysym::Tab)
        .on_press(move || {
            let Some(id) = history.lock().unwrap().previous else {
                return;
            };
            // The window may have closed since; then nothing matches and we do nothing.
            let Some(win) = window::get_all().find(|w| w.id() == id) else {
                return;
            };
            focus_and_reveal(&win);
        })
        .group("Window")
        .description("Focus the previous window");
}

fn focus_and_reveal(win: &WindowHandle) {
    if !win.is_on_active_tag()
        && let Some(tag) = win.tags().next()
    {
        tag.switch_to();
    }
    win.set_focused(true);
    win.raise();
}
