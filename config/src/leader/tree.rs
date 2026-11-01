//! The menu tree: which key opens which submenu or runs which action.
//!
//! Keys are characters as printed on the keycap (layout-applied), `' '` is Space.
//! Uppercase letters are distinct keys (Shift+letter).

use std::sync::Arc;

use pinnacle_api::output;
use pinnacle_api::pinnacle;
use pinnacle_api::tag::{self, TagHandle};
use pinnacle_api::window::{self, WindowHandle};

use crate::layouts;
use crate::settings::{Settings, sh};

pub type Run = Arc<dyn Fn() + Send + Sync>;

pub enum Entry {
    Menu {
        key: char,
        label: String,
        children: Vec<Entry>,
    },
    Action {
        key: char,
        label: String,
        run: Run,
    },
}

impl Entry {
    pub fn key(&self) -> char {
        match self {
            Entry::Menu { key, .. } | Entry::Action { key, .. } => *key,
        }
    }
}

pub fn menu(key: char, label: &str, children: Vec<Entry>) -> Entry {
    Entry::Menu {
        key,
        label: label.to_string(),
        children,
    }
}

pub fn action(key: char, label: &str, run: impl Fn() + Send + Sync + 'static) -> Entry {
    Entry::Action {
        key,
        label: label.to_string(),
        run: Arc::new(run),
    }
}

/// Runs a shell command line.
fn cmd(key: char, label: &str, command_line: impl ToString) -> Entry {
    let command_line = command_line.to_string();
    action(key, label, move || sh(&command_line))
}

/// Follows `path` from the root; `None` if a key does not lead to a submenu.
pub fn children_at<'a>(root: &'a [Entry], path: &[char]) -> Option<&'a [Entry]> {
    let mut entries = root;
    for key in path {
        match entries.iter().find(|e| e.key() == *key)? {
            Entry::Menu { children, .. } => entries = children,
            Entry::Action { .. } => return None,
        }
    }
    Some(entries)
}

/// Every action with its key path, e.g. ("l n", "layout › Next layout"). For the palette.
pub fn flatten(root: &[Entry]) -> Vec<(String, String, Run)> {
    fn walk(entries: &[Entry], keys: &str, labels: &str, out: &mut Vec<(String, String, Run)>) {
        for entry in entries {
            let key = if entry.key() == ' ' {
                "SPC".to_string()
            } else {
                entry.key().to_string()
            };
            let keys = if keys.is_empty() {
                key
            } else {
                format!("{keys} {key}")
            };
            match entry {
                Entry::Menu {
                    label, children, ..
                } => {
                    let labels = if labels.is_empty() {
                        label.clone()
                    } else {
                        format!("{labels} › {label}")
                    };
                    walk(children, &keys, &labels, out);
                }
                Entry::Action { label, run, .. } => {
                    let label = if labels.is_empty() {
                        label.clone()
                    } else {
                        format!("{labels} › {label}")
                    };
                    out.push((keys, label, run.clone()));
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(root, "", "", &mut out);
    out
}

pub fn build(s: &Settings) -> Vec<Entry> {
    let terminal = s.terminal.clone();
    let browser = s.file.str("apps.browser", "");
    let files = s.file.str("apps.file_manager", "");

    vec![
        menu(
            'o',
            "open",
            vec![
                cmd('t', "Terminal", &terminal),
                cmd(
                    'b',
                    "Browser",
                    or_default(&browser, "xdg-open https://duckduckgo.com"),
                ),
                cmd(
                    'f',
                    "File manager",
                    or_default(&files, "xdg-open \"$HOME\""),
                ),
                cmd('l', "App launcher", &s.launcher),
            ],
        ),
        menu(
            'h',
            "help",
            vec![action('o', "Keybind overlay", show_overlay)],
        ),
        menu(
            'w',
            "window",
            vec![
                action('c', "Close", || with_focused(|w| w.close())),
                action('f', "Toggle floating", || {
                    with_focused(|w| {
                        w.toggle_floating();
                        w.raise();
                    })
                }),
                action('F', "Toggle fullscreen", || {
                    with_focused(|w| {
                        w.toggle_fullscreen();
                        w.raise();
                    })
                }),
                action('m', "Toggle maximized", || {
                    with_focused(|w| {
                        w.toggle_maximized();
                        w.raise();
                    })
                }),
                menu('t', "move to tag", per_tag(s, move_focused_to)),
            ],
        ),
        menu('t', "tag", {
            let mut entries = per_tag(s, |t| t.switch_to());
            entries.push(action('n', "Next tag", || step_tag(1)));
            entries.push(action('p', "Previous tag", || step_tag(-1)));
            entries
        }),
        menu(
            'l',
            "layout",
            vec![
                action('n', "Next layout", || layouts::cycle(true)),
                action('p', "Previous layout", || layouts::cycle(false)),
                action('m', "Master left", || layouts::set("master-left")),
                action('M', "Master right", || layouts::set("master-right")),
                action('d', "Dwindle", || layouts::set("dwindle")),
                action('f', "Fair", || layouts::set("fair")),
            ],
        ),
        menu(
            'q',
            "session",
            vec![
                action('r', "Reload config", pinnacle::reload_config),
                action('q', "Quit prompt", || {
                    pinnacle_api::snowcap::QuitPrompt::new().show()
                }),
            ],
        ),
        menu(
            's',
            "system",
            vec![
                action('s', "Screenshot", crate::plugins::screenshot::take),
                action(
                    'd',
                    "Toggle max / preferred refresh rate",
                    toggle_refresh_rate,
                ),
                cmd(
                    'v',
                    "Toggle mute",
                    "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle",
                ),
                cmd('n', "Dismiss notifications", "makoctl dismiss --all"),
            ],
        ),
    ]
}

fn or_default(setting: &str, default: &str) -> String {
    if setting.is_empty() {
        default.to_string()
    } else {
        setting.to_string()
    }
}

fn show_overlay() {
    pinnacle_api::snowcap::BindOverlay::new().show();
}

fn with_focused(f: impl FnOnce(&WindowHandle)) {
    if let Some(win) = window::get_focused() {
        f(&win);
    }
}

/// One entry per tag name on the focused output (keys 1–9).
fn per_tag(s: &Settings, f: fn(&TagHandle)) -> Vec<Entry> {
    s.tag_names
        .iter()
        .filter_map(|name| {
            let key = name.chars().next().filter(|_| name.chars().count() == 1)?;
            let name = name.to_string();
            Some(action(key, &format!("Tag {name}"), move || {
                if let Some(tag) = focused_tag_named(&name) {
                    f(&tag);
                }
            }))
        })
        .collect()
}

fn focused_tag_named(name: &str) -> Option<TagHandle> {
    output::get_focused()
        .and_then(|o| tag::get_on_output(name, &o))
        .or_else(|| tag::get(name))
}

fn move_focused_to(tag: &TagHandle) {
    with_focused(|w| w.move_to_tag(tag));
}

/// Switches to the tag `delta` places after the active one on the focused output (wrapping).
fn step_tag(delta: isize) {
    let Some(out) = output::get_focused() else {
        return;
    };
    let tags: Vec<TagHandle> = out.tags().collect();
    let Some(current) = tags.iter().position(|t| t.active()) else {
        return;
    };
    let len = tags.len() as isize;
    let next = (current as isize + delta).rem_euclid(len) as usize;
    tags[next].switch_to();
}

/// Switches the focused monitor between its fastest mode and its preferred (often 60 Hz) mode.
fn toggle_refresh_rate() {
    let Some(out) = output::get_focused() else {
        return;
    };
    let (Some(current), Some(preferred)) = (out.current_mode(), out.preferred_mode()) else {
        return;
    };
    let fastest = out
        .modes()
        .filter(|m| m.size == preferred.size)
        .max_by_key(|m| m.refresh_rate_mhz);
    let target = match fastest {
        Some(fast) if current != fast => fast,
        _ => preferred,
    };
    out.set_mode(target.size.w, target.size.h, target.refresh_rate_mhz);
}
