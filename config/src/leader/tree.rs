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
/// Text shown in brackets after a menu's key in the breadcrumb, computed on every redraw.
pub type Badge = Arc<dyn Fn() -> Option<String> + Send + Sync>;

pub enum Entry {
    Menu {
        key: char,
        label: String,
        children: Vec<Entry>,
        badge: Option<Badge>,
        /// Shown in the panel's footer while this menu is open.
        hint: Option<String>,
    },
    Action {
        key: char,
        label: String,
        run: Run,
        /// Works and shows up in the palette, but is not listed in the panel.
        hidden: bool,
    },
}

impl Entry {
    pub fn key(&self) -> char {
        match self {
            Entry::Menu { key, .. } | Entry::Action { key, .. } => *key,
        }
    }

    fn with_badge(mut self, f: impl Fn() -> Option<String> + Send + Sync + 'static) -> Self {
        if let Entry::Menu { badge, .. } = &mut self {
            *badge = Some(Arc::new(f));
        }
        self
    }

    fn with_hint(mut self, text: &str) -> Self {
        if let Entry::Menu { hint, .. } = &mut self {
            *hint = Some(text.to_string());
        }
        self
    }

    fn hidden(mut self) -> Self {
        if let Entry::Action { hidden, .. } = &mut self {
            *hidden = true;
        }
        self
    }
}

pub fn menu(key: char, label: &str, children: Vec<Entry>) -> Entry {
    Entry::Menu {
        key,
        label: label.to_string(),
        children,
        badge: None,
        hint: None,
    }
}

pub fn action(key: char, label: &str, run: impl Fn() + Send + Sync + 'static) -> Entry {
    Entry::Action {
        key,
        label: label.to_string(),
        run: Arc::new(run),
        hidden: false,
    }
}

/// Runs a shell command line.
fn cmd(key: char, label: &str, command_line: impl ToString) -> Entry {
    let command_line = command_line.to_string();
    action(key, label, move || sh(&command_line))
}

/// Runs a script from this repo's `scripts/` folder with arguments.
fn script(key: char, label: &str, path: &str, args: &[&str]) -> Entry {
    let program = settings::repo_dir().join("scripts").join(path);
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    action(key, label, move || {
        pinnacle_api::process::Command::new(program.to_string_lossy())
            .args(&args)
            .spawn();
    })
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
    let agent_dir = s.file.path("agent.dir", "~/agent").display().to_string();
    let default_backend = s.file.str("agent.backend", "claude");

    vec![
        script('C', "Capture a thought", "capture/quick.sh", &[]),
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
            'f',
            "file",
            vec![
                script('f', "Find file (browse folders)", "files/browse.sh", &[]),
                script('F', "Fuzzy-find any file", "files/open.sh", &[]),
                script('r', "Recent files", "files/open.sh", &["--recent"]),
                script('c', "Edit this config", "files/edit-config.sh", &[]),
                script(
                    's',
                    "Edit local.toml (settings)",
                    "files/edit-config.sh",
                    &["--local"],
                ),
            ],
        ),
        menu(
            'n',
            "notes",
            vec![
                script('c', "Capture in Helix", "capture/edit.sh", &[]),
                script('i', "Open inbox", "capture/search.sh", &["--open"]),
                script('s', "Search captures", "capture/search.sh", &[]),
                script('t', "Triage inbox (local model)", "capture/triage.sh", &[]),
            ],
        ),
        menu(
            'a',
            "agent",
            vec![
                script(
                    'b',
                    "Toggle backend (Claude / local)",
                    "agent/backend.sh",
                    &["--toggle"],
                ),
                script('r', "Research thread", "agent/thread.sh", &["research"]),
                script(
                    'f',
                    "Feature in a repo (plan mode)",
                    "agent/feature.sh",
                    &[],
                ),
                script('g', "HTML slides", "agent/thread.sh", &["slides"]),
                script(
                    'w',
                    "Ask about the window",
                    "agent/handoff.sh",
                    &["--window"],
                ),
                script('s', "Ask about a region", "agent/handoff.sh", &["--region"]),
                script('a', "Ask about a file", "agent/handoff.sh", &["--file"]),
                script(
                    'c',
                    "Ask about the clipboard",
                    "agent/handoff.sh",
                    &["--clipboard"],
                ),
                script(
                    'p',
                    "Switch Claude profile",
                    "agent/profile.sh",
                    &["--switch"],
                ),
                script('n', "New Claude profile", "agent/profile.sh", &["--new"]),
                cmd(
                    'o',
                    "Open agent folder",
                    format!(
                        "mkdir -p {dir} && {opener} {dir}",
                        dir = shell_quote(&agent_dir),
                        opener = or_default(&files, "xdg-open"),
                    ),
                ),
                // Shift+letter: the same entry, once, on the other backend.
                script(
                    'R',
                    "Research thread (other backend)",
                    "agent/thread.sh",
                    &["research", "--other"],
                )
                .hidden(),
                script(
                    'F',
                    "Feature in a repo (other backend)",
                    "agent/feature.sh",
                    &["--other"],
                )
                .hidden(),
                script(
                    'G',
                    "HTML slides (other backend)",
                    "agent/thread.sh",
                    &["slides", "--other"],
                )
                .hidden(),
                script(
                    'W',
                    "Ask about the window (other backend)",
                    "agent/handoff.sh",
                    &["--window", "--other"],
                )
                .hidden(),
                script(
                    'S',
                    "Ask about a region (other backend)",
                    "agent/handoff.sh",
                    &["--region", "--other"],
                )
                .hidden(),
                script(
                    'A',
                    "Ask about a file (other backend)",
                    "agent/handoff.sh",
                    &["--file", "--other"],
                )
                .hidden(),
                script(
                    'C',
                    "Ask about the clipboard (other backend)",
                    "agent/handoff.sh",
                    &["--clipboard", "--other"],
                )
                .hidden(),
            ],
        )
        .with_badge(move || Some(agent_backend(&default_backend)))
        .with_hint("Shift+letter: run once on the other backend"),
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

/// The agent backend Menu → a b last chose (state file written by scripts/agent/backend.sh),
/// else the one from settings.
fn agent_backend(default: &str) -> String {
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".local/state"))
        });
    state
        .and_then(|dir| std::fs::read_to_string(dir.join("pinnacle-config/agent-backend")).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// Quotes a string for `sh -c`.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn tree() -> Vec<Entry> {
        build(&Settings::for_test())
    }

    fn find(entries: &[Entry], key: char) -> &Entry {
        entries.iter().find(|e| e.key() == key).expect("key exists")
    }

    /// Two entries with one key would make the second unreachable.
    #[test]
    fn keys_are_unique_at_every_level() {
        fn check(entries: &[Entry], path: &str) {
            let mut seen = HashSet::new();
            for entry in entries {
                assert!(
                    seen.insert(entry.key()),
                    "duplicate key {:?} in SPC {path}",
                    entry.key()
                );
                if let Entry::Menu { children, key, .. } = entry {
                    check(children, &format!("{path}{key} "));
                }
            }
        }
        check(&tree(), "");
    }

    #[test]
    fn children_at_follows_menus_and_stops_at_actions() {
        let tree = tree();
        let agent = children_at(&tree, &['a']).expect("a is a menu");
        assert!(agent.iter().any(|e| e.key() == 'r'));
        assert!(
            children_at(&tree, &['a', 'r']).is_none(),
            "a r is an action"
        );
        assert!(children_at(&tree, &['!']).is_none(), "unknown key");
        assert_eq!(children_at(&tree, &[]).map(<[_]>::len), Some(tree.len()));
    }

    #[test]
    fn flatten_lists_key_paths_and_labels() {
        let tree = vec![
            action(' ', "Palette", || {}),
            menu('l', "layout", vec![action('n', "Next layout", || {})]),
        ];
        let rows: Vec<(String, String)> =
            flatten(&tree).into_iter().map(|(k, l, _)| (k, l)).collect();
        assert_eq!(
            rows,
            [
                ("SPC".into(), "Palette".into()),
                ("l n".into(), "layout › Next layout".into()),
            ]
        );
    }

    /// Shift+letter in the agent menu runs the lowercase entry on the other backend.
    #[test]
    fn agent_shift_entries_are_hidden_twins_of_lowercase_ones() {
        let tree = tree();
        let agent = children_at(&tree, &['a']).unwrap();
        let shifted: Vec<&Entry> = agent.iter().filter(|e| e.key().is_uppercase()).collect();
        assert!(!shifted.is_empty());
        for entry in shifted {
            let Entry::Action { hidden, key, .. } = entry else {
                panic!("{:?} should be an action", entry.key());
            };
            assert!(hidden, "{key} should be hidden from the panel");
            let lower = key.to_ascii_lowercase();
            assert!(matches!(
                find(agent, lower),
                Entry::Action { hidden: false, .. }
            ));
        }
    }

    #[test]
    fn agent_menu_shows_backend_badge_and_shift_hint() {
        let tree = tree();
        let Entry::Menu { badge, hint, .. } = find(&tree, 'a') else {
            panic!("a should be a menu");
        };
        assert!(badge.as_ref().and_then(|b| b()).is_some());
        assert!(hint.as_deref().unwrap_or("").contains("Shift"));
    }

    #[test]
    fn shell_quote_survives_single_quotes() {
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
        assert_eq!(shell_quote("/a b"), "'/a b'");
    }
}
