//! `pctl` – query and control a running Pinnacle session from the command line.
//!
//! Every query prints JSON on stdout so scripts and Claude Code can parse it.
//! Diagnostics go to stderr. Exit code is non-zero on errors.
//!
//! It connects through the same gRPC socket the config uses. Inside a Pinnacle session the socket
//! path is in `PINNACLE_GRPC_SOCKET`; if that is missing (e.g. over SSH), pctl looks for the
//! newest `pinnacle-grpc-*.sock` in `$XDG_RUNTIME_DIR`.

use std::path::PathBuf;
use std::process::ExitCode;

use pinnacle_api::input::{BindInfo, BindInfoKind};
use pinnacle_api::output::{self, OutputHandle};
use pinnacle_api::process::Command;
use pinnacle_api::tag::{self, TagHandle};
use pinnacle_api::window::{self, WindowHandle};
use serde_json::{Value, json};

const SOCKET_ENV: &str = "PINNACLE_GRPC_SOCKET";

const USAGE: &str = "\
pctl – query and control Pinnacle

USAGE:
    pctl <COMMAND> [ARGS]

QUERIES (print JSON):
    state                 Outputs, tags and windows
    windows               Windows only
    focused               The focused window, or null
    binds                 Key and mouse binds with mods, group and description

ACTIONS:
    focus <window-id>     Focus a window (switches to its tag if hidden)
    close <window-id>     Close a window
    float <window-id>     Toggle floating
    move <window-id> <tag>  Move a window to a tag
    tag <name>            Switch the focused output to a tag
    mode <output> <w>x<h>@<hz>  Set an output's mode, e.g. `mode DP-3 1920x1080@144`
                          (until next login; picks the closest listed refresh rate)
    spawn <cmd> [args..]  Start a program inside the session
    reload                Restart the config (rebuilds it via cargo run)
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() || matches!(args[0].as_str(), "-h" | "--help" | "help") {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }

    if std::env::var_os(SOCKET_ENV).is_none() {
        match find_socket() {
            // SAFETY: we are still single-threaded; the async runtime starts afterwards.
            Some(path) => unsafe { std::env::set_var(SOCKET_ENV, path) },
            None => {
                eprintln!("pctl: no Pinnacle socket found. Is Pinnacle running?");
                return ExitCode::FAILURE;
            }
        }
    }

    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("pctl: {msg}");
            ExitCode::FAILURE
        }
    }
}

fn find_socket() -> Option<PathBuf> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from)?;
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            name.starts_with("pinnacle-grpc-") && name.ends_with(".sock")
        })
        .max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok())
        .map(|e| e.path())
}

fn run(args: Vec<String>) -> Result<(), String> {
    // The API's sync methods need a multi-threaded Tokio runtime (they use `block_in_place`).
    let runtime = pinnacle_api::tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("could not start async runtime: {e}"))?;

    runtime.block_on(async move {
        connect_quietly().await?;
        dispatch(&args)
    })
}

/// `pinnacle_api::connect` prints a status line to stdout, which would corrupt our JSON.
/// Point stdout at stderr while connecting, then restore it.
async fn connect_quietly() -> Result<(), String> {
    // SAFETY: plain fd juggling on the process's own stdout/stderr.
    let saved = unsafe { libc::dup(libc::STDOUT_FILENO) };
    if saved >= 0 {
        unsafe { libc::dup2(libc::STDERR_FILENO, libc::STDOUT_FILENO) };
    }

    let result = pinnacle_api::connect().await.map_err(|e| e.to_string());

    if saved >= 0 {
        unsafe {
            libc::dup2(saved, libc::STDOUT_FILENO);
            libc::close(saved);
        }
    }
    result
}

fn dispatch(args: &[String]) -> Result<(), String> {
    let arg = |i: usize, what: &str| {
        args.get(i)
            .map(String::as_str)
            .ok_or_else(|| format!("missing argument: {what}"))
    };

    match args[0].as_str() {
        "state" => print_json(json!({
            "outputs": output::get_all().map(|o| output_json(&o)).collect::<Vec<_>>(),
            "windows": window::get_all().map(|w| window_json(&w)).collect::<Vec<_>>(),
        })),
        "windows" => print_json(Value::Array(
            window::get_all().map(|w| window_json(&w)).collect(),
        )),
        "focused" => print_json(
            window::get_focused()
                .map(|w| window_json(&w))
                .unwrap_or(Value::Null),
        ),
        "binds" => print_json(Value::Array(
            pinnacle_api::input::bind_infos()
                .map(|b| bind_json(&b))
                .collect(),
        )),
        "focus" => {
            let win = find_window(arg(1, "window-id")?)?;
            if !win.is_on_active_tag()
                && let Some(tag) = win.tags().next()
            {
                tag.switch_to();
            }
            win.set_focused(true);
            win.raise();
        }
        "close" => find_window(arg(1, "window-id")?)?.close(),
        "float" => find_window(arg(1, "window-id")?)?.toggle_floating(),
        "move" => {
            let win = find_window(arg(1, "window-id")?)?;
            let tag = find_tag(arg(2, "tag")?)?;
            win.move_to_tag(&tag);
        }
        "tag" => find_tag(arg(1, "tag")?)?.switch_to(),
        "mode" => set_mode(arg(1, "output")?, arg(2, "mode")?)?,
        "spawn" => {
            let program = arg(1, "cmd")?;
            Command::new(program)
                .args(&args[2..])
                .spawn()
                .ok_or_else(|| format!("could not spawn {program}"))?;
        }
        "reload" => pinnacle_api::pinnacle::reload_config(),
        other => return Err(format!("unknown command `{other}` (try `pctl help`)")),
    }
    Ok(())
}

/// Parses `WxH@HZ` and sets the listed mode whose refresh rate is closest to HZ.
fn set_mode(name: &str, spec: &str) -> Result<(), String> {
    let bad = || format!("`{spec}` is not WxH@HZ, e.g. 1920x1080@144");
    let (size, hz) = spec.split_once('@').ok_or_else(bad)?;
    let (w, h) = size.split_once('x').ok_or_else(bad)?;
    let (w, h): (u32, u32) = (w.parse().map_err(|_| bad())?, h.parse().map_err(|_| bad())?);
    let mhz = (hz.parse::<f64>().map_err(|_| bad())? * 1000.0) as u32;

    let out = output::get_by_name(name).ok_or_else(|| format!("no output named {name}"))?;
    let mode = out
        .modes()
        .filter(|m| m.size.w == w && m.size.h == h)
        .min_by_key(|m| m.refresh_rate_mhz.abs_diff(mhz))
        .ok_or_else(|| format!("{name} has no {w}x{h} mode"))?;
    out.set_mode(w, h, mode.refresh_rate_mhz);
    print_json(mode_json(&mode));
    Ok(())
}

fn find_window(id: &str) -> Result<WindowHandle, String> {
    let id: u32 = id
        .parse()
        .map_err(|_| format!("`{id}` is not a window id"))?;
    window::get_all()
        .find(|w| w.id() == id)
        .ok_or_else(|| format!("no window with id {id}"))
}

/// Prefer the tag on the focused output, since every output has its own "1".."9".
fn find_tag(name: &str) -> Result<TagHandle, String> {
    output::get_focused()
        .and_then(|o| tag::get_on_output(name, &o))
        .or_else(|| tag::get(name))
        .ok_or_else(|| format!("no tag named {name}"))
}

fn output_json(o: &OutputHandle) -> Value {
    json!({
        "name": o.name(),
        "make": o.make(),
        "model": o.model(),
        "focused": o.focused(),
        "enabled": o.enabled(),
        "scale": o.scale(),
        "loc": o.loc().map(|p| json!({"x": p.x, "y": p.y})),
        "logical_size": o.logical_size().map(|s| json!({"w": s.w, "h": s.h})),
        "mode": o.current_mode().map(|m| mode_json(&m)),
        "modes": o.modes().map(|m| mode_json(&m)).collect::<Vec<_>>(),
        "tags": o.tags().map(|t| json!({
            "id": t.id(),
            "name": t.name(),
            "active": t.active(),
        })).collect::<Vec<_>>(),
    })
}

fn mode_json(m: &output::Mode) -> Value {
    json!({"w": m.size.w, "h": m.size.h, "refresh_mhz": m.refresh_rate_mhz})
}

fn window_json(w: &WindowHandle) -> Value {
    json!({
        "id": w.id(),
        "app_id": w.app_id(),
        "title": w.title(),
        "focused": w.focused(),
        "floating": w.floating(),
        "fullscreen": w.fullscreen(),
        "maximized": w.maximized(),
        "visible": w.is_on_active_tag(),
        "output": w.output().map(|o| o.name()),
        "tags": w.tags().map(|t| t.name()).collect::<Vec<_>>(),
        "loc": w.loc().map(|p| json!({"x": p.x, "y": p.y})),
        "size": w.size().map(|s| json!({"w": s.w, "h": s.h})),
    })
}

fn bind_json(b: &BindInfo) -> Value {
    let (kind, key) = match &b.kind {
        BindInfoKind::Key { xkb_name, .. } => ("key", xkb_name.clone()),
        BindInfoKind::Mouse { button } => ("mouse", format!("{button:?}")),
    };
    json!({
        "kind": kind,
        "key": key,
        // pinnacle-api v0.2.4 reports ignored ISO level 3/5 mods as required ones.
        "mods": b.mods.iter_names().map(|(name, _)| name).collect::<Vec<_>>(),
        "layer": b.layer.name(),
        "group": b.group,
        "description": b.description,
    })
}

fn print_json(value: Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(&value).expect("JSON serialization cannot fail")
    );
}
