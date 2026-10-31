# pinnacle-config

My [Pinnacle](https://github.com/pinnacle-comp/pinnacle) setup for Fedora Workstation 44 and
Pop!_OS 24.04: a Rust-configured, dynamically tiling Wayland compositor that runs alongside the
existing desktop and is built to be changed live — by hand or by Claude Code.

- **Live reconfiguration:** the config is a separate Rust program. `Mod+Ctrl+R` (or `pctl reload`)
  rebuilds and restarts it; the compositor and your windows keep running.
- **Fail-safe:** if the config fails to build or crashes, Pinnacle falls back to its built-in config.
- **Runtime control:** `pctl` prints the session state as JSON and can focus, move and close windows.
- **Extensible:** features live in `config/src/plugins/`, one module each.
- **Claude Code ready:** `CLAUDE.md` describes the workflow and rules; `.claude/settings.json`
  allows the safe commands and asks before destructive ones.

## Install

```sh
git clone git@github.com:lycheese/pinnacle-config.git ~/src/pinnacle-config
cd ~/src/pinnacle-config
./scripts/install-fedora.sh          # Fedora 44
./scripts/install-popos.sh           # Pop!_OS 24.04 / Ubuntu 24.04 (installs Rust via rustup)
./scripts/nested.sh                  # try it in a window inside your desktop (Mod = Alt)
```

Both installers accept `--no-lua` (skip `pinnacle client`) and `--packages-only`.
Pop!_OS 22.04 is not supported: its libraries are too old.

Then log out, choose **Pinnacle** from the session menu on the login screen, and log in.
Your existing desktop (GNOME or COSMIC) stays installed and selectable.

## Default binds

`Mod` is Super in a real session and Alt when nested. `Mod+S` shows all binds.

| Bind | Action |
|---|---|
| `Mod+Return` / `Mod+D` | Terminal (Alacritty) / launcher (fuzzel) |
| `Mod+H/J/K/L` | Focus left/down/up/right |
| `Mod+Shift+H/J/K/L` | Swap with window in that direction |
| `Mod+Tab` | Back to the previous window |
| `Mod+Space` / `Mod+Shift+Space` | Next / previous layout (master-stack left/right, dwindle, fair) |
| `Mod+1…9` | Switch to tag |
| `Mod+Shift+1…9` | Move window to tag |
| `Mod+Ctrl+1…9` | Also show tag |
| `Mod+F` / `Mod+M` / `Mod+Ctrl+Space` | Fullscreen / maximize / float |
| `Mod+Shift+C` | Close window |
| `Mod+Ctrl+R` | Reload config |
| `Mod+Shift+Q` | Quit prompt (`Mod+Ctrl+Shift+Q` quits immediately) |

## Working with Claude Code

Open Claude Code in this repo from a terminal *inside* the Pinnacle session, so `pctl` reaches it.
Examples:

- "Float all Nautilus windows and pin Firefox to tag 2."
- "Add a scratchpad terminal plugin on Mod+grave."
- "Write a layout where the master window takes 60% on my ultrawide output only."
- "Show me what's on tag 3" → Claude runs `pctl state`.

For compositor changes, Claude works in `~/src/pinnacle` and tests nested first (see `CLAUDE.md`).

## Updating Pinnacle

Pinnacle's API is pre-1.0 and changes between releases. To update:
1. Read the [migration notes](https://github.com/pinnacle-comp/pinnacle/blob/main/wiki/configuration/breaking-changes-and-migrations.md).
2. Bump the tag in `Cargo.toml` and run `PINNACLE_REF=<new tag> ./scripts/install-<distro>.sh`.
3. `cargo check --workspace`, fix what broke, reload.
