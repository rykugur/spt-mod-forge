# SPT Mod Forge

A quick-and-easy terminal user interface (TUI) mod manager for [SPTarkov](https://sp-tarkov.com/), built in Rust with [Ratatui](https://ratatui.rs/).

It discovers mods via the [SPT Forge API](https://forge.sp-tarkov.com), lets you toggle them with a single key, and performs clean install/uninstall on "commit" by tracking exactly which files belong to each mod.

## Features

- **Split-pane TUI**: Browse curated and searched mods on the left, see rich details on the right.
- **Forge API powered**: Full metadata, versions, compatibility filtering, and direct downloads (no local mod definitions in v1).
- **Desired state + commit flow**: Space to toggle "desired" enabled state. Press `c` to review the diff and apply changes.
- **Precise install/uninstall**: Records every file and directory written per mod. Uninstall removes *only* those paths (best-effort directory cleanup). Shared files between mods are handled gracefully.
- **Smart caching**: Optional on-disk cache for mod lists and downloads (`~/.cache/spt-mod-forge` or `$SPT_CACHE_DIR`). Toggleable and force-refreshable (`f` or command palette).
- **Token UX**: `FORGE_API_TOKEN` is read from the environment only (never written to any file, never read from CWD). The app prompts on first use if missing.
- **Theming**: Default "terminal" mode respects your terminal colors (`Color::Reset`). Also supports popular schemes: Catppuccin, Tokyo Night, Dracula, etc. Hot-reloadable in the settings modal.
- **Braille/unicode animations**: Smooth spinners during network and I/O (via the `rattles` crate). Toggleable.
- **Power-user ergonomics**:
  - `:` (or Ctrl-k) fuzzy command palette
  - `hjkl` + arrow keys everywhere
  - Pop-over settings modal (`s`) with live apply
  - Full keyboard-driven workflow
- **XDG compliant + strict no-CWD policy**: All state and config lives in `~/.config/spt-mod-forge` and cache dir. The app never touches the current working directory.
- **Nix first**: Reproducible builds and shells via flakes. Works as a `nix run` target or as a flake input.

## Requirements

- Linux (primary target)
- A working SPTarkov installation (classically `~/Games/SPTarkov`)
- A Forge API token (free account at https://forge.sp-tarkov.com)

## Quick Start

### Run directly (recommended)

```bash
# One-shot (uses the published flake)
nix run github:rykugur/spt-mod-forge

# Or from a local checkout
nix run .#spt-mod-forge
```

### As a flake input

```nix
{
  inputs.spt-mod-forge.url = "github:rykugur/spt-mod-forge";
  # ...
}
```

Then `spt-mod-forge.packages.${system}.default` or similar.

### First run

1. Set your token (recommended for the shell session or permanently):
   ```bash
   export FORGE_API_TOKEN="your-token-here"
   ```
2. Run the binary.
3. If no SPT path is found, the app will prompt you (or set `SPT_PATH` / edit `config.toml`).

The app will create its config and state database under XDG directories on first use.

## Key Bindings (v1)

| Key          | Action                          |
|--------------|---------------------------------|
| `↑` / `k`    | Move selection up               |
| `↓` / `j`    | Move selection down             |
| `Space`      | Toggle desired state for mod    |
| `c` / `Enter`| Open commit review & apply      |
| `:`          | Open command palette (fuzzy)    |
| `Ctrl-k`     | Alternative palette trigger     |
| `s`          | Open settings modal             |
| `f`          | Force refresh mod lists (bypass cache) |
| `q` / `Esc`  | Quit (or close modal)           |
| `?` / `h`    | Help / keybindings              |

Inside the command palette you can type things like:
- `refresh lists (force)`
- `open settings`
- `toggle cache`
- `set color_scheme dracula`
- `commit changes`
- `quit`

## Configuration

Location: `~/.config/spt-mod-forge/config.toml` (or `$XDG_CONFIG_HOME/spt-mod-forge/config.toml`)

Example:

```toml
[spt]
path = ""   # Leave empty to use ~/Games/SPTarkov or SPT_PATH env

[cache]
enabled = true
dir = ""    # Leave empty for XDG cache or use SPT_CACHE_DIR env

[ui]
animations = true
default_sort = "most_downloaded"   # most_downloaded | recently_updated | my_list
color_scheme = "terminal"          # terminal | catppuccin | tokyonight | dracula | ...
```

Environment variable precedence (highest first):
- `FORGE_API_TOKEN`
- `SPT_PATH`
- `SPT_CACHE_DIR`
- Values from `config.toml`
- Built-in defaults

The design document (`docs/superpowers/specs/2026-06-05-spt-mod-forge-design.md`) and implementation plan contain the full rationale and locked decisions.

## Development

```bash
# Enter the development shell (recommended)
nix develop

# Common commands
cargo run
cargo test --lib
cargo clippy -- -D warnings
nix build .#spt-mod-forge
```

The project follows a detailed 18-task TDD plan (see `docs/superpowers/plans/...`).

## License

This project is currently unlicensed / all rights reserved by the author. Check back for a formal license.

## Acknowledgments

- SPT Forge team for the excellent API
- The Ratatui and Rust TUI community
- The `rattles` crate for beautiful braille spinners

---

**Status**: Fully implemented and functional (see design doc for details).

Happy modding!
