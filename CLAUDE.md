# CLAUDE.md

This repo is a personal config for the **Pinnacle** Wayland compositor (Rust, Smithay-based,
AwesomeWM-style dynamic tiling with tags) on Fedora Workstation 44 or Pop!_OS 24.04. It is symlinked to
`~/.config/pinnacle`. Pinnacle runs `cargo run --package pinnacle-config` (see `pinnacle.toml`)
on startup and on every reload; the config talks to the compositor over gRPC.

## Layout

| Path | What |
|---|---|
| `config/src/settings.rs` | Mod key, terminal, launcher, gaps, tag names |
| `config/src/binds.rs` | Compositor, window, app and media binds |
| `config/src/layouts.rs` | Layout generators and layout cycling |
| `config/src/tags.rs` | Tags 1–9 and their binds |
| `config/src/plugins/` | Self-contained features, one module each, registered in `plugins/mod.rs` |
| `waybar/config.jsonc` | Waybar modules (style is the system default); restart waybar to apply |
| `pctl/` | CLI that queries/controls the running session and prints JSON |
| `scripts/install-fedora.sh`, `scripts/install-popos.sh` | Distro packages and toolchain, then the shared steps |
| `scripts/lib/common.sh` | Shared install steps: build Pinnacle, install session files, link this repo |
| `scripts/nested.sh` | Runs Pinnacle as a window inside the current desktop with this config |

The compositor source lives in `~/src/pinnacle` (not in this repo).

## API reference

- Rust API docs: https://pinnacle-comp.github.io/rust-reference/main
- The API is pinned to tag `v0.2.4` in `Cargo.toml`. The exact source is in
  `~/.cargo/git/checkouts/pinnacle-*/` after a build, or in `~/src/pinnacle/api/rust/src`.
  **Read the source there before using an API function you have not seen in this repo**;
  the API is pre-1.0 and online examples are often outdated.
- Upstream reference config: `~/src/pinnacle/api/rust/examples/default_config/main.rs`.

## Workflow for config changes

1. Edit the code.
2. `cargo check --workspace` and `cargo clippy --workspace` – fix all errors and warnings.
3. `cargo build --package pinnacle-config` – so the reload does not stall on compiling.
4. `pctl reload` – restarts the config; the compositor and all windows keep running.
5. Verify with `pctl state` (JSON) that the change did what was intended, e.g. a window rule
   applied or tags exist. Tell the user what to try by hand for keybinds.

If the new config panics, Pinnacle falls back to its built-in default config and the error shows
on the next successful start (Snowcap crash message). Check `journalctl --user -u pinnacle -n 100`
when running as a session.

## Runtime control (IPC)

- `pctl state | windows | focused | binds` – JSON snapshot. Window `id`s are what the action commands take.
- `pctl focus|close|float <id>`, `pctl move <id> <tag>`, `pctl tag <name>`, `pctl spawn <cmd>`,
  `pctl reload`.
- `pinnacle client -e '<lua>'` – the full Lua API for one-off calls not covered by pctl, e.g.
  `pinnacle client -e 'print(Output.get_focused().name)'`. Only available if the Lua API was
  installed.
- pctl finds the session via `PINNACLE_GRPC_SOCKET`, or the newest socket in `$XDG_RUNTIME_DIR`.
- Prefer extending `pctl` over long Lua one-liners when a query will be reused.

## Writing a plugin

Create `config/src/plugins/<name>.rs` with a top doc comment (what it does, which binds it adds) and
`pub fn setup(settings: &Settings)`, then register it in `plugins/mod.rs`. Keep plugins independent;
shared state lives in `Arc<Mutex<_>>` inside the plugin. Every bind needs `.group()` and
`.description()` so it appears in the keybind overlay.

## Hacking the compositor itself

- Work in `~/src/pinnacle` on a branch (ideally a fork; set `PINNACLE_REPO` for the install script).
- Build with `just build` and test **nested** first: `scripts/nested.sh ~/src/pinnacle/target/debug/pinnacle`.
- Only install a modified compositor (re-run the install script for this distro with `PINNACLE_REF=<branch>`)
  after it works nested. If you change the protobuf API, the `pinnacle-api` dependency here must
  point at the same branch.

## Rules

- Never remove or rebind the quit (`Mod+Shift+Q`, `Mod+Ctrl+Shift+Q`) or reload (`Mod+Ctrl+R`) binds.
- Do not change `pinnacle.toml`'s `run` command or the `pinnacle-api` tag without asking; the tag
  must match the installed compositor version.
- Do not run `pctl close`, `pinnacle client` calls that quit, or anything that kills the session
  without asking: the user may be working in those windows.
- Do not run `sudo` or the install script without asking.
- Commit each working change separately with a clear message so it can be reverted.
