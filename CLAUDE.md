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
| `settings.toml`, `local.toml` | Documented defaults; gitignored personal overrides. Scripts read them via `eval "$(pctl settings --env)"` |
| `settings/` | Crate that merges the two, used by the config and pctl |
| `waybar/config.jsonc.in` | Waybar template, rendered with the settings at config start; restart waybar to apply. Tags are `custom/tag-N` modules fed by `pctl tags --watch --tag N`, styled by their `active`/`occupied` classes in `waybar/style.css` (imports the system default style). Its Font Awesome icons are invisible private-use characters: edit with a script and check their count stays the same |
| `pctl/` | CLI that queries/controls the running session and prints JSON |
| `scripts/install-fedora.sh`, `scripts/install-popos.sh` | Distro packages and toolchain, then the shared steps |
| `scripts/lib/common.sh` | Shared install steps: build Pinnacle, install session files, link this repo |
| `scripts/nested.sh` | Runs Pinnacle as a window inside the current desktop with this config |
| `tests/run.sh` | Runs every test (see Testing); CI runs it too |
| `tests/shell/` | Shell tests: `test_*` functions in `test_*.sh`, run by `tests/shell/run.sh` |
| `tests/python/` | Python tests (stdlib `unittest`) for the Python scripts |

The compositor source lives in `~/src/pinnacle` (not in this repo).

## API reference

- Rust API docs: https://pinnacle-comp.github.io/rust-reference/main
- The API is pinned in `Cargo.toml` to the `fix/signal-stream-end` branch of the fork
  github.com/lycheese/pinnacle: v0.2.4 plus a layer-shell fix and an API signal-loop fix, with
  the same protocol as v0.2.4. The exact source is in `~/.cargo/git/checkouts/pinnacle-*/`
  after a build, or in `~/src/pinnacle/api/rust/src` on that branch.
  **Read the source there before using an API function you have not seen in this repo**;
  the API is pre-1.0 and online examples are often outdated.
- Upstream reference config: `~/src/pinnacle/api/rust/examples/default_config/main.rs`.

## Workflow for config changes

1. Write a failing test, then the code, red/green (see Testing).
2. `tests/run.sh` – all suites green.
3. `cargo check --workspace` and `cargo clippy --workspace --all-targets` – fix all errors and warnings.
4. `cargo build --package pinnacle-config` – so the reload does not stall on compiling.
5. `pctl reload` – restarts the config; the compositor and all windows keep running.
6. Verify with `pctl state` (JSON) that the change did what was intended, e.g. a window rule
   applied or tags exist. Tell the user what to try by hand for keybinds.

If the new config panics, Pinnacle falls back to its built-in default config and the error shows
on the next successful start (Snowcap crash message). Check `journalctl --user -u pinnacle -n 100`
when running as a session.

## Testing (red/green TDD, mandatory)

Every change is test-driven:

1. **Red:** write a test for the new behaviour first and run it. See it fail, for the reason you
   expect. A test that passes straight away tests nothing new: fix the test.
2. **Green:** make the smallest change that makes it pass.
3. **Refactor** while everything stays green.

Every bug fix starts with a test that reproduces the bug (red) before the fix. Mention the
red run in the commit message or report. Keep tests fast and hermetic: no running compositor,
network, real `$HOME` or `local.toml`.

Where tests live:
- **Rust:** `#[cfg(test)] mod tests` next to the code. `config::Settings::for_test()` gives the
  `settings.toml` defaults without a compositor; `settings::Settings::parse` takes TOML text.
  Pull logic out of compositor calls into plain functions so it can be tested (e.g. pctl's
  `SpinCheck::record`, the waybar JSON builders).
- **Shell:** `tests/shell/test_*.sh`; each `test_*` function runs in a subshell with a throwaway
  `$HOME`, state and runtime folder, and settings from `settings.toml` only. Use the
  `assert_*` helpers in `tests/shell/run.sh`; stub external tools (notify-send, terminals) with
  small scripts on `$PATH`.
- **Python:** `tests/python/test_*.py`, stdlib `unittest`; patch out network calls.

Run everything with `tests/run.sh`, or one suite: `cargo test -p <crate>`,
`PCTL=$PWD/target/debug/pctl tests/shell/run.sh [name-filter]`,
`python3 -m unittest discover -s tests/python`.

Compositor behaviour (keybinds, focus, rendering, the menu panel) cannot be unit-tested. Test the
logic behind it, then still do the manual check from the workflow above and tell the user what to
try by hand.

## Runtime control (IPC)

- `pctl state | windows | focused | binds` – JSON snapshot. Window `id`s are what the action commands take.
- `pctl tags [--watch] [--tag <name>]` – active and occupied tags as waybar custom-module JSON
  (`text`/`tooltip`/`class`, plus `outputs` or `tag`). `--watch` prints a line per change: signals
  for tag switches and window open/close/focus, a 2 s re-check for window moves (no signal).
  After changing pctl, `cargo install --path pctl` and restart waybar so its watchers use the new binary.
- `pctl focus|close|float <id>`, `pctl move <id> <tag>`, `pctl tag <name>`, `pctl spawn <cmd>`,
  `pctl mode <output> <w>x<h>@<hz>` (until next login), `pctl reload`.
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
- Do not change `pinnacle.toml`'s `run` command or the `pinnacle-api` pin without asking; it
  must speak the same protocol as the installed compositor.
- Do not run `pctl close`, `pinnacle client` calls that quit, or anything that kills the session
  without asking: the user may be working in those windows.
- Do not run `sudo` or the install script without asking.
- Commit each working change separately with a clear message so it can be reverted.
- No change without a test first (red/green, see Testing); `tests/run.sh` must pass before a commit.
