//! Leader menu (Doom Emacs / Spacemacs style): Mod+Space opens a which-key panel at the bottom;
//! single keys walk into submenus or run actions. The tree lives in `tree.rs`.
//!
//! How keys reach the panel: the panel is a Snowcap layer with exclusive keyboard focus. While it
//! is open the bind layer `"leader"` is active, so the default-layer binds (including the
//! `super_grab` catch-all) are off and keys pressed with Mod still held reach the panel.
//! Failsafes: Escape and Mod+Space are compositor binds in that layer, and `main.rs` resets the
//! bind layer on every config start. Quit and reload binds work in every layer.
//!
//! Binds: Mod+Space (open/close).

mod popup;
pub mod tree;

use std::sync::{Arc, Mutex};

use pinnacle_api::input::{self, Bind, BindLayer, Keysym, Mod};
use pinnacle_api::output;
use snowcap_api::layer::{self, Anchor, ExclusiveZone, KeyboardInteractivity, LayerHandle, ZLayer};

use crate::settings::Settings;
use popup::{Nav, Popup, key_name};
use tree::{Entry, children_at};

const LAYER: &str = "leader";

/// Any modifier state.
const ANY_MODS: Mod = Mod::IGNORE_SHIFT
    .union(Mod::IGNORE_CTRL)
    .union(Mod::IGNORE_ALT)
    .union(Mod::IGNORE_SUPER)
    .union(Mod::IGNORE_ISO_LEVEL3_SHIFT)
    .union(Mod::IGNORE_ISO_LEVEL5_SHIFT);

static OPEN: Mutex<Option<LayerHandle<()>>> = Mutex::new(None);

pub fn setup(s: &Settings) {
    let mut entries = tree::build(s);
    let actions = Arc::new(tree::flatten(&entries));
    entries.insert(
        0,
        tree::action(' ', "Palette: search all actions", move || {
            palette(&actions)
        }),
    );
    let tree = Arc::new(entries);

    input::keybind(s.mod_key, Keysym::space)
        .on_press(move || open(tree.clone()))
        .group("Compositor")
        .description("Leader menu (which-key)");

    let leader = BindLayer::get(LAYER);
    leader
        .keybind(ANY_MODS, Keysym::Escape)
        .on_press(close)
        .group("Leader")
        .description("Close the leader menu");
    leader
        .keybind(s.mod_key, Keysym::space)
        .on_press(close)
        .group("Leader")
        .description("Close the leader menu");
}

fn open(tree: Arc<Vec<Entry>>) {
    let mut slot = OPEN.lock().unwrap();
    if slot.is_some() {
        return;
    }

    let width = output::get_focused()
        .and_then(|o| o.logical_size())
        .map(|size| size.w as f32)
        .unwrap_or(1280.0);
    let nav = Arc::new(Mutex::new(Nav::default()));
    let popup = Popup {
        nav: nav.clone(),
        tree: tree.clone(),
        width,
    };

    BindLayer::get(LAYER).enter();
    let handle = match layer::new_widget(
        popup,
        Some(Anchor::Bottom),
        KeyboardInteractivity::Exclusive,
        ExclusiveZone::Respect,
        ZLayer::Overlay,
    ) {
        Ok(handle) => handle,
        Err(err) => {
            eprintln!("leader: could not open the panel: {err}");
            BindLayer::DEFAULT.enter();
            return;
        }
    };

    handle.on_key_press(move |handle, key, _mods| on_key(&handle, &tree, &nav, key));
    *slot = Some(handle);
}

fn close() {
    if let Some(handle) = OPEN.lock().unwrap().take() {
        handle.close();
    }
    BindLayer::DEFAULT.enter();
}

/// Lists every action ("l n   layout › Next layout") in fuzzel and runs the chosen one.
/// Runs on Tokio's blocking pool, not a plain thread: the action may call the Pinnacle API,
/// whose sync methods need the runtime (`Handle::current()` panics on a plain thread).
fn palette(actions: &Arc<Vec<(String, String, tree::Run)>>) {
    let actions = actions.clone();
    pinnacle_api::tokio::task::spawn_blocking(move || {
        let lines: String = actions
            .iter()
            .map(|(keys, label, _)| format!("{keys:<8} {label}\n"))
            .collect();
        let Ok(mut child) = std::process::Command::new("fuzzel")
            .args(["--dmenu", "--index", "--width", "60"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
        else {
            eprintln!("leader: fuzzel not found");
            return;
        };
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(lines.as_bytes());
        }
        let Ok(out) = child.wait_with_output() else {
            return;
        };
        let chosen = String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse::<usize>()
            .ok();
        if let Some((_, _, run)) = chosen.and_then(|i| actions.get(i)) {
            run();
        }
    });
}

fn on_key(handle: &LayerHandle<()>, tree: &[Entry], nav: &Mutex<Nav>, key: Keysym) {
    if key.is_modifier_key() {
        return;
    }
    let mut state = nav.lock().unwrap();
    state.note = None;

    if key == Keysym::BackSpace {
        state.path.pop();
        drop(state);
        handle.send_message(());
        return;
    }
    if key == Keysym::Escape {
        drop(state);
        close();
        return;
    }

    let pressed = if key == Keysym::space {
        Some(' ')
    } else {
        key.key_char()
    };
    let found = pressed.and_then(|c| {
        children_at(tree, &state.path)?
            .iter()
            .find(|entry| entry.key() == c)
    });

    match found {
        Some(Entry::Menu { key, .. }) => {
            state.path.push(*key);
            drop(state);
            handle.send_message(());
        }
        Some(Entry::Action { run, .. }) => {
            let run = run.clone();
            drop(state);
            close();
            run();
        }
        None => {
            if let Some(c) = pressed {
                state.note = Some(format!("no entry for {}", key_name(c)));
            }
            drop(state);
            handle.send_message(());
        }
    }
}
