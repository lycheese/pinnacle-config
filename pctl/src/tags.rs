//! `pctl tags [--watch] [--tag <name>]`: active and occupied tags, in a shape waybar's
//! `custom` modules read directly (`"return-type": "json"`).
//!
//! Each line is one JSON object with `text`, `tooltip` and `class` for waybar, plus `outputs`
//! (or `tag` with `--tag`) for scripts. With `--watch` a new line is printed whenever the result
//! changes. Tag switches, window open/close and focus changes arrive as compositor signals; a
//! window moving to another tag has no signal, so the state is also re-checked every 2 seconds.

use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use pinnacle_api::output;
use pinnacle_api::signal::{OutputSignal, TagSignal, WindowSignal};
use pinnacle_api::tag;
use pinnacle_api::tokio::sync::Notify;
use pinnacle_api::tokio::time::{sleep, timeout};
use pinnacle_api::window;
use serde_json::{Value, json};

const RECHECK: Duration = Duration::from_secs(2);
/// Signals come in bursts (a tag switch deactivates one tag and activates another).
const SETTLE: Duration = Duration::from_millis(30);

struct Tag {
    name: String,
    active: bool,
    /// App ids of the windows on this tag.
    apps: Vec<String>,
}

struct Output {
    name: String,
    focused: bool,
    tags: Vec<Tag>,
}

pub async fn run(args: &[String]) -> Result<(), String> {
    let mut watch = false;
    let mut only: Option<String> = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--watch" | "-w" => watch = true,
            "--tag" => only = Some(args.next().ok_or("--tag needs a tag name")?.clone()),
            other => return Err(format!("unknown argument for tags: `{other}`")),
        }
    }

    let render = |outputs: &[Output]| match &only {
        Some(name) => single_tag_json(outputs, name),
        None => all_tags_json(outputs),
    };

    if !watch {
        print_line(&render(&snapshot()));
        return Ok(());
    }

    let changed = Arc::new(Notify::new());
    connect_signals(&changed);

    let mut last = String::new();
    loop {
        let line = render(&snapshot()).to_string();
        if line != last {
            print_line(&line);
            last = line;
        }
        // A timeout just means "re-check now".
        let _ = timeout(RECHECK, changed.notified()).await;
        sleep(SETTLE).await;
    }
}

fn connect_signals(changed: &Arc<Notify>) {
    let ping = || {
        let changed = changed.clone();
        move || changed.notify_one()
    };
    let p = ping();
    tag::connect_signal(TagSignal::Active(Box::new(move |_, _| p())));
    let p = ping();
    tag::connect_signal(TagSignal::Created(Box::new(move |_| p())));
    let p = ping();
    tag::connect_signal(TagSignal::Removed(Box::new(move |_| p())));
    let p = ping();
    window::connect_signal(WindowSignal::Created(Box::new(move |_| p())));
    let p = ping();
    window::connect_signal(WindowSignal::Destroyed(Box::new(move |_, _, _| p())));
    // Moving a window off the visible tag usually changes focus, so this catches most moves.
    let p = ping();
    window::connect_signal(WindowSignal::Focused(Box::new(move |_| p())));
    let p = ping();
    output::connect_signal(OutputSignal::Focused(Box::new(move |_| p())));
    let p = ping();
    output::connect_signal(OutputSignal::Connect(Box::new(move |_| p())));
    let p = ping();
    output::connect_signal(OutputSignal::Disconnect(Box::new(move |_| p())));
}

fn snapshot() -> Vec<Output> {
    let mut apps: HashMap<u32, Vec<String>> = HashMap::new();
    for win in window::get_all() {
        let app = win.app_id();
        for tag in win.tags() {
            apps.entry(tag.id()).or_default().push(app.clone());
        }
    }

    output::get_all()
        .filter(|o| o.enabled())
        .map(|o| Output {
            name: o.name(),
            focused: o.focused(),
            tags: o
                .tags()
                .map(|t| Tag {
                    name: t.name(),
                    active: t.active(),
                    apps: apps.remove(&t.id()).unwrap_or_default(),
                })
                .collect(),
        })
        .collect()
}

/// `[1] 2 5`: active tags in brackets and bold, then tags with windows. With several outputs
/// each gets its own group, prefixed with its name.
fn all_tags_json(outputs: &[Output]) -> Value {
    let groups: Vec<String> = outputs
        .iter()
        .map(|o| {
            let tags: Vec<String> = o
                .tags
                .iter()
                .filter(|t| t.active || !t.apps.is_empty())
                .map(|t| label(t.active, &t.name))
                .collect();
            let tags = tags.join(" ");
            if outputs.len() > 1 {
                format!("{}: {tags}", escape(&o.name))
            } else {
                tags
            }
        })
        .collect();

    let tooltip: Vec<String> = outputs
        .iter()
        .flat_map(|o| {
            o.tags.iter().filter(|t| !t.apps.is_empty()).map(|t| {
                let output = if outputs.len() > 1 {
                    format!("{} ", escape(&o.name))
                } else {
                    String::new()
                };
                format!(
                    "{output}{}: {}",
                    label(t.active, &t.name),
                    escape(&t.apps.join(", "))
                )
            })
        })
        .collect();

    json!({
        "text": groups.join("  │  "),
        "tooltip": tooltip.join("\n"),
        "class": "tags",
        "outputs": outputs.iter().map(|o| json!({
            "name": o.name,
            "focused": o.focused,
            "tags": o.tags.iter().map(|t| json!({
                "name": t.name,
                "active": t.active,
                "windows": t.apps.len(),
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

/// One tag name across all outputs, for one waybar module per tag. The text is empty (waybar
/// hides the module) unless the tag is active or has windows on some output.
fn single_tag_json(outputs: &[Output], name: &str) -> Value {
    let tags: Vec<&Tag> = outputs
        .iter()
        .flat_map(|o| &o.tags)
        .filter(|t| t.name == name)
        .collect();
    let active = tags.iter().any(|t| t.active);
    let apps: Vec<&str> = tags
        .iter()
        .flat_map(|t| &t.apps)
        .map(String::as_str)
        .collect();

    let text = if active || !apps.is_empty() {
        label(active, name)
    } else {
        String::new()
    };

    let mut class = vec!["tag"];
    if active {
        class.push("active");
    }
    if !apps.is_empty() {
        class.push("occupied");
    }

    json!({
        "text": text,
        "tooltip": escape(&apps.join("\n")),
        "class": class,
        "tag": {"name": name, "active": active, "windows": apps.len()},
    })
}

/// Pango markup for one tag: `<b>[2]</b>` when active, `5` when it only has windows.
fn label(active: bool, name: &str) -> String {
    if active {
        format!("<b>[{}]</b>", escape(name))
    } else {
        escape(name)
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn print_line(line: &impl std::fmt::Display) {
    let mut out = std::io::stdout().lock();
    // waybar reads line by line, so flush every update. A closed pipe means waybar is gone.
    if writeln!(out, "{line}").and_then(|()| out.flush()).is_err() {
        std::process::exit(0);
    }
}
