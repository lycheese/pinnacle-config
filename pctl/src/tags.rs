//! `pctl tags [--watch] [--tag <name>]`: active and occupied tags, in a shape waybar's
//! `custom` modules read directly (`"return-type": "json"`).
//!
//! Each line is one JSON object with `text`, `tooltip` and `class` for waybar, plus `outputs`
//! (or `tag` with `--tag`) for scripts. With `--watch` a new line is printed whenever the result
//! changes. Tag switches, window open/close and focus changes arrive as compositor signals; a
//! window moving to another tag has no signal, so the state is also re-checked every 2 seconds.
//!
//! A config reload ends every signal stream, and pinnacle-api v0.2.4 then spins on the closed
//! stream at full CPU instead of stopping. The watcher notices the CPU use and restarts itself
//! (same process ID and stdout, so waybar keeps reading), which also connects to the new signals.

use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};

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

    // On a plain thread: the spinning tasks hog the async runtime, so a check there never runs.
    std::thread::spawn(|| {
        let mut spin = SpinCheck::new();
        loop {
            std::thread::sleep(RECHECK);
            if spin.spinning() {
                restart();
            }
        }
    });

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

/// Detects the busy loop left behind by a config reload (see the module docs): the process
/// used more than half a CPU core since the last check, while it should be idle.
struct SpinCheck {
    wall: Instant,
    cpu: Option<Duration>,
}

impl SpinCheck {
    fn new() -> Self {
        Self {
            wall: Instant::now(),
            cpu: cpu_time(),
        }
    }

    fn spinning(&mut self) -> bool {
        self.record(Instant::now(), cpu_time())
    }

    /// Takes a reading (wall clock, process CPU time); true if CPU time grew by more than half
    /// the wall time since the previous reading.
    fn record(&mut self, wall: Instant, cpu: Option<Duration>) -> bool {
        let elapsed = wall.saturating_duration_since(self.wall);
        let before = std::mem::replace(&mut self.cpu, cpu);
        self.wall = wall;
        matches!((before, cpu), (Some(b), Some(n)) if n.saturating_sub(b) * 2 > elapsed)
    }
}

/// CPU time used by this process (user + system), from /proc/self/stat.
fn cpu_time() -> Option<Duration> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    // Fields after the parenthesised command name; utime and stime are fields 14 and 15.
    let fields: Vec<&str> = stat.rsplit_once(')')?.1.split_whitespace().collect();
    let ticks: u64 = fields.get(11)?.parse::<u64>().ok()? + fields.get(12)?.parse::<u64>().ok()?;
    // Linux reports these in USER_HZ, which is 100 on every common architecture.
    Some(Duration::from_millis(ticks * 10))
}

/// Replaces this process with a fresh `pctl` run with the same arguments. argv[0] is looked up
/// on PATH again, so a `cargo install`ed newer pctl takes over too.
fn restart() -> ! {
    use std::os::unix::process::CommandExt;
    let mut args = std::env::args_os();
    let program = args.next().unwrap_or_else(|| "pctl".into());
    let err = std::process::Command::new(program).args(args).exec();
    eprintln!("pctl: could not restart after a config reload: {err}");
    std::process::exit(1);
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

/// One tag name across all outputs, for one waybar module per tag. The text is the plain name,
/// or empty (waybar hides the module) unless the tag is active or has windows on some output.
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

    // Plain name: waybar/style.css styles the active/occupied classes.
    let text = if active || !apps.is_empty() {
        escape(name)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(name: &str, active: bool, apps: &[&str]) -> Tag {
        Tag {
            name: name.into(),
            active,
            apps: apps.iter().map(|a| a.to_string()).collect(),
        }
    }

    fn output(name: &str, tags: Vec<Tag>) -> Output {
        Output {
            name: name.into(),
            focused: true,
            tags,
        }
    }

    #[test]
    fn all_tags_shows_active_and_occupied_only() {
        let outputs = [output(
            "DP-3",
            vec![
                tag("1", false, &[]),
                tag("2", true, &[]),
                tag("3", false, &["firefox", "kitty"]),
            ],
        )];
        let json = all_tags_json(&outputs);
        assert_eq!(json["text"], "<b>[2]</b> 3");
        assert_eq!(json["tooltip"], "3: firefox, kitty");
        assert_eq!(json["outputs"][0]["tags"][2]["windows"], 2);
    }

    #[test]
    fn all_tags_prefixes_outputs_when_there_are_several() {
        let outputs = [
            output("DP-1", vec![tag("1", true, &[])]),
            output("DP-2", vec![tag("1", true, &["a"])]),
        ];
        assert_eq!(
            all_tags_json(&outputs)["text"],
            "DP-1: <b>[1]</b>  │  DP-2: <b>[1]</b>"
        );
    }

    #[test]
    fn single_tag_classes_and_empty_text_when_unused() {
        let outputs = [output(
            "DP-3",
            vec![
                tag("1", true, &[]),
                tag("2", false, &["mpv"]),
                tag("3", false, &[]),
            ],
        )];
        let one = single_tag_json(&outputs, "1");
        assert_eq!(one["text"], "1");
        assert_eq!(one["class"], json!(["tag", "active"]));
        let two = single_tag_json(&outputs, "2");
        assert_eq!(two["class"], json!(["tag", "occupied"]));
        assert_eq!(two["tooltip"], "mpv");
        let three = single_tag_json(&outputs, "3");
        assert_eq!(three["text"], "");
        assert_eq!(three["class"], json!(["tag"]));
    }

    #[test]
    fn pango_markup_is_escaped() {
        assert_eq!(escape("a<b>&c"), "a&lt;b&gt;&amp;c");
        let outputs = [output("x", vec![tag("<1>", false, &["R&D"])])];
        assert_eq!(all_tags_json(&outputs)["tooltip"], "&lt;1&gt;: R&amp;D");
    }

    fn reading(cpu_ms: u64) -> Option<Duration> {
        Some(Duration::from_millis(cpu_ms))
    }

    #[test]
    fn cpu_time_is_readable() {
        assert!(cpu_time().is_some(), "/proc/self/stat is readable");
    }

    #[test]
    fn spin_check_is_quiet_while_idle() {
        let start = Instant::now();
        let mut spin = SpinCheck {
            wall: start,
            cpu: reading(1000),
        };
        assert!(!spin.record(start + Duration::from_secs(2), reading(1010)));
    }

    #[test]
    fn spin_check_notices_a_busy_loop() {
        let start = Instant::now();
        let mut spin = SpinCheck {
            wall: start,
            cpu: reading(1000),
        };
        // Two busy threads for 2 s: 4 s of CPU.
        assert!(spin.record(start + Duration::from_secs(2), reading(5000)));
        // The next interval is measured from the last reading.
        assert!(!spin.record(start + Duration::from_secs(4), reading(5020)));
    }

    #[test]
    fn spin_check_ignores_missing_readings() {
        let start = Instant::now();
        let mut spin = SpinCheck {
            wall: start,
            cpu: None,
        };
        assert!(!spin.record(start + Duration::from_secs(2), reading(9000)));
    }
}
