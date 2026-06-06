# SPT Mod Forge - Design Document

**Date**: 2026-06-05  
**Status**: Implemented (working TUI; see end of doc for details) 2026-06-05  
**Visual Reference**: http://localhost:64616/ (companion mockups for split view, commit flow, config, token UX, etc.)

## 1. Overview and Goals

SPT Mod Forge is a quick-and-easy TUI mod manager for SPTarkov, built in Rust with Ratatui.

Core goals (from initial requirements):
- Present a list of mods (initially driven by SPT Forge API).
- Allow enable/disable with Space (affects "desired" state).
- "Commit" action diffs desired vs. currently installed state and performs install/uninstall.
- Track exactly which files were installed per mod so uninstalls are clean.
- Use Forge API for metadata (name, description, versions, compatibility, download links, source links).
- Support pre-defined/curated mods via API (local YAML/JSON overrides deferred to future).
- No files ever written to or expected in the current working directory (CWD). All user data in XDG config/cache dirs.

Non-goals for v1 (deferred):
- Full local mod definition files (YAML/JSON) with overrides.
- Multi-install support (separate state per SPT).
- Favorites (simple enhancement later).
- Advanced dependency resolution or virus scanning.

The app is packaged as a binary (via the existing flake: `nix run ...#spt-mod-forge` or as a dependency).

## 2. UI Layout and Interaction

**Primary Layout**: Split-pane (chosen over single-pane for better information density while browsing).

- **Left pane**: Scrollable list of mods.
  - Toggle indicator (✓ / space for desired state).
  - Mod name + short version.
  - Status badge: ENABLED / DISABLED / PENDING (install or uninstall).
  - Sorts: "Most Downloaded", "Recently Updated", "My List" (personal managed mods).
- **Right pane**: Details for selected mod (always visible).
  - Name, version, author, download count, last updated.
  - Links: Forge page, GitHub/source.
  - Description/teaser.
  - Current status + installed version vs. latest compatible.
  - Tracked installed files (from state DB).
  - Dependencies (if known from API).
- **Footer / Key hints**: ↑↓ select, Space toggle desired, Enter/details, c commit (with pending count), f force refresh lists, q quit, ? help, etc.
- **Search**: Type to filter or search Forge API to add new mods to "My List".

**Commit Flow** (simple & direct):
- 'c' computes diff (desired vs. currently installed per state DB).
- Shows review screen: TO INSTALL (with version + file count), TO UNINSTALL (with file count), summary, warnings (e.g. "requires restart").
- Confirmation: y to proceed, n/Esc to cancel, d for dry-run (no changes).
- During apply: Progress with braille/unicode spinner (via rattles), per-mod status (downloading, extracting, recording paths, removing).
- On success: Summary. Errors reported per-mod; continue with others where possible (simple direct approach).
- No automatic rollback for partial failures in v1 (user can re-commit to clean up).

Animations (braille/unicode spinners) are shown during any I/O: list loading, search, download, extract, commit progress. Implemented via `rattles` crate (minimal, FOSS, Ratatui example, braille presets). Toggleable via config.

**Theming / Color Schemes** (new requirement):
- Default ("terminal"): Respect the user's terminal colors as much as possible. Use Ratatui's default styles and `Color::Reset` where appropriate so the terminal emulator's theme (foreground, background, etc.) shines through. Minimal forced colors for accessibility and user preference.
- Predefined popular themes: catppuccin (mocha/latte/etc.), tokyonight, dracula, and others (gruvbox, nord, solarized, rose-pine, etc. can be added easily).
- Configurable via `config.toml`:
  ```toml
  [ui]
  color_scheme = "terminal"   # "terminal" | "catppuccin" | "tokyonight" | "dracula" | ...
  ```
- Implementation: A `Theme` struct defining a small palette (primary fg/bg, accent, warning, success, etc.). Ratatui `Style` and `Color` (RGB + ANSI support). "terminal" mode avoids overriding fg/bg aggressively. Themes can be hot-reloaded or switched in future TUI settings.
- Animations/spinners adapt to the current theme's accent color where sensible.
- Future: User-defined custom themes via extra TOML snippets or files.

**Navigation**:
- Both arrow keys (↑↓←→) and vim/helix-style hjkl movement are supported everywhere lists or selections are present (mod list, settings, command menu, etc.).
- This provides familiarity for both general users and vim users without forcing one style.

**Command Menu / Palette** (new requirement, approved):
- Fuzzy-searchable command menu (like VS Code command palette or vim's : commands).
- Triggered by pressing `:` (preferred, vim/helix style) or `Ctrl-k` (web-style alternative; both supported).
- Opens a modal overlay with an input field at top.
- As you type, it fuzzy-filters a list of available commands using simple substring + basic scoring for v1 (fast, no heavy deps).
- Arrow keys or j/k to navigate results; Enter to execute the selected command (basic argument support from day one, e.g. ":set theme dracula").
- Esc to close without action.
- Initial v1 commands (locked from approved mockup examples):
  - refresh lists (force)
  - open settings (the modal)
  - toggle cache
  - set color_scheme <name>
  - commit changes
  - quit
  - help (or open keybindings view)
- Context-sensitive commands when a mod is selected (future extension, e.g. "enable/disable this mod", "view on Forge").
- This centralizes power-user actions and keeps the main UI clean (no need for dozens of single-key hotkeys).

**Settings View** (chosen: pop-over modal):
- Accessed via `s` key (or via command menu: "open settings").
- Appears as a pop-over modal overlay centered on the screen (not full replacement of the main view, to keep context).
- Simple list or form of current settings (color scheme, animations, cache enabled, default sort, etc.).
- Navigation with arrows/hjkl.
- Inline editing: arrows or numbers to cycle values (e.g. color schemes), Enter to confirm change (immediate apply where possible, e.g. theme switch, animation toggle).
- Esc or `q` to close (changes are saved on close if modified).
- This keeps advanced configuration discoverable inside the app without leaving the TUI or editing files manually (though manual editing of config.toml is still fully supported and takes precedence with env vars).
- Future: more settings (e.g. custom keybindings) can be added here.

## 3. Mod Discovery and Population

**Hybrid approach** (chosen):
- Start with curated lists populated by querying Forge API.
  - "Most Downloaded" (default).
  - "Recently Updated".
  - "My List" (mods the user has added via search or curated).
- Search bar: Queries Forge API (with SPT version filter) to discover and add mods to the personal list.
- "My List" persists in state DB; curated sorts are dynamic (with cache).

Mods are identified primarily by Forge `id` (numeric) + GUID fallback. Metadata (name, description, versions, links, compatibility) comes live from Forge API (no local YAML/JSON definitions in v1).

SPT version filtering is applied using the resolved SPT install version (see section 6).

## 4. Configuration

**Mechanism**: Small human-editable `config.toml` in the XDG config directory (`~/.config/spt-mod-forge/config.toml`). (Chosen over pure SQLite for human readability and dotfiles-friendliness. Env vars always take precedence.)

**Initial keys** (approved):

```toml
[spt]
# Optional. If empty we fall back to ~/Games/SPTarkov + prompt + SPT_PATH env
path = ""

[cache]
# Master toggle for downloads + list query results caching
enabled = true

# Custom cache root. Empty = XDG default ($XDG_CACHE_HOME/spt-mod-forge or ~/.cache/spt-mod-forge)
# Can also be overridden at runtime via SPT_CACHE_DIR env
dir = ""

[ui]
# Enable/disable braille/unicode spinners (via rattles) during long operations
animations = true

# Default sort for the curated lists in the left pane
default_sort = "most_downloaded"   # or "recently_updated"

# Color scheme. "terminal" respects your terminal's own colors (recommended default).
# Other options: "catppuccin", "tokyonight", "dracula", etc.
color_scheme = "terminal"
```

**Loading order / precedence**:
1. Environment variables (SPT_PATH, SPT_CACHE_DIR, FORGE_API_TOKEN, etc.).
2. Values from config.toml.
3. Hardcoded XDG defaults.

The config dir and file are created on first run if missing (with sensible defaults). No files are ever created or read from the CWD.

**Token (FORGE_API_TOKEN)**:
- Only supported via environment variable `FORGE_API_TOKEN`.
- Never stored in config.toml or any file we create (secrets hygiene).
- On first run or when missing: Interactive TUI prompt (chosen: C) that asks for the token value for immediate use.
  - If user provides it → use for the session.
  - If user skips/cancels → hard error + clear instructions and exit (fallback A).
- Clear messaging in prompt + errors + --help + docs:
  - "Set the FORGE_API_TOKEN environment variable before running (e.g. export in your shell profile, or use direnv/a secret manager)."
- We never create a .env file (in CWD or config dir) or write the token for the user.
- Users who want file-based secrets manage it themselves outside our app.

## 5. API Integration

The app relies on the SPT Forge API for all mod metadata, versions, compatibility, descriptions, and download/source links. (Local overrides deferred.)

**Key Endpoints Used** (based on public Forge API v0 patterns):
- Search/list mods with filters (query, spt_version, sort by downloads/recent).
- Get specific mod by ID or GUID, including versions and source links.
- Possibly batch updates or dependencies in future.

**Authentication**:
- Bearer token via `FORGE_API_TOKEN` env var (as designed).
- Prompt on first run if missing.
- Include in all API requests; handle 401/403 by re-prompting or erroring.

**SPT Version Handling**:
- Detect from the resolved SPT install (e.g., parse from `SPTarkov.Server.Core.dll` version, or a version.txt/manifest in the dir).
- Pass to API queries for compatible results only (e.g., `filter[spt_version]=4.0.13`).

**Caching**:
- List results cached (as in section 8), with force refresh.
- Individual mod metadata cached briefly (e.g., session or short TTL) to reduce API calls during browsing.
- Download archives cached in the cache dir (respecting the cache toggle).

**Rate Limiting & Resilience**:
- Respect any `Retry-After` or rate limit headers from the API.
- Simple exponential backoff on transient errors (network, 5xx).
- User-facing: show "API rate limited, retrying..." with spinner.
- Offline/limited mode: use cached data only; disable search/add until token/API available.
- No aggressive polling; only on user actions (load list, search, commit, refresh).

**Downloads**:
- Use the direct link from the chosen mod version in the API response (often GitHub releases or direct Forge host).
- Stream to cache file; support common formats (zip primary; 7z if needed via dep later).
- During commit: show per-mod progress (bytes, status) with animation.
- Verify basic integrity if API provides hashes (future).

**Error Handling**:
- Network/auth errors: clear messages, option to re-enter token.
- Incompatible version: warn in details pane.
- Download failures: skip that mod in commit, continue others, report at end.

This keeps the app responsive and respectful of the API while providing the core value.

## 6. SPT Installation Path

**Default (Linux-focused)**: `~/Games/SPTarkov`

**Logic**:
1. Check `SPT_PATH` env var (if set, use it — enables targeting different installs).
2. Else default `~/Games/SPTarkov`.
3. Validate it looks like a valid SPT install (contains `user/mods` and `BepInEx`).
4. If not found or invalid → interactive prompt for full path. Offer to remember (write to config.toml).
5. Use the resolved path for:
   - Detecting SPT version (for API filters and compatibility).
   - Install/uninstall targets.
   - File path recording (relative to this root).

Multi-install support (isolated state per SPT install) is deferred as a future enhancement. The current design (global state + env override) allows light multi-install use but shares the desired state across targets.

## 6. State Management

**Location**: `~/.config/spt-mod-forge/state.db` (SQLite). (Chosen: user config dir only + SQLite for reliability with relational "mod owns many files" data.)

**Core tables** (simple & practical):

- `managed_mods`
  - forge_id (INTEGER PK)
  - guid (TEXT, unique fallback)
  - name, last_known_version (denormalized from last API fetch for display)
  - desired_enabled (BOOLEAN)
  - last_installed_version (TEXT, NULL if never)
  - added_at (TIMESTAMP)

- `installed_files`
  - id (INTEGER PK)
  - forge_id (INTEGER FK)
  - relative_path (TEXT) — e.g. "user/mods/SAIN/..." or "BepInEx/plugins/..."
  - is_directory (BOOLEAN)
  - installed_at (TIMESTAMP)
  - Unique on (forge_id, relative_path)

**Usage**:
- On "commit": Compare `desired_enabled` vs. what is currently recorded as installed.
- Newly desired + not installed → install + INSERT paths.
- No longer desired + installed → uninstall (delete exactly the recorded paths) + DELETE rows.
- "My List" is the set of rows in managed_mods (populated via curated or search/add).

SQLite provides transactions, easy queries ("who owns this path?"), and robustness. A small `config` table can hold runtime mirrors of settings if needed.

No JSON for the main state (SQLite chosen).

## 8. Caching

**Scope**: 
- Downloaded mod archives.
- Forge API list query results ("most downloaded", "recently updated" blobs).

**Location**: `$XDG_CACHE_HOME/spt-mod-forge` (or `~/.cache/spt-mod-forge`), with subdirs `downloads/` and `lists/`.

**Config**:
- `cache.enabled` (bool, default true) — persisted in config.toml, toggleable in TUI.
- `cache.dir` (string, optional override) — also overridable at runtime via `SPT_CACHE_DIR` env.
- No TTL (user controls freshness via force refresh or disabling cache). SPT/mod dev cycles are slow and irregular; users decide when to refresh.

**Behavior**:
- On list load/search: Check cache first (if enabled). Use cached if present.
- `f` / force refresh key: Bypass list cache for that operation, re-query API, update cache.
- On install: If archive is cached for the exact version, use it; else download + cache.
- Cache is per-user, survives app restarts.

## 9. Install / Uninstall Logic (Simple & Direct)

Chosen: Simple & direct (A).

**On enable + commit for a mod**:
- Determine best compatible version from Forge API (using resolved SPT version).
- Download the archive (link from API) — respect cache if enabled.
- Extract the archive.
- Walk the extracted contents; collect entries under `user/mods/` and/or `BepInEx/`.
- Extract/write only those, relative to the resolved SPT root.
- Record **every** concrete path written (files + directories) in `installed_files` table.
- Store the installed version.

**On disable + commit for a mod**:
- Look up the exact list of paths previously recorded for this mod.
- Delete the files first.
- Then attempt to rmdir the directories (best effort; ignore if not empty or errors).
- Remove the rows from `installed_files`.

**Safety / notes**:
- Never delete anything outside the recorded paths for that mod.
- Paths are always relative to the SPT root we resolved at install time.
- If a path was also recorded by another mod, it will be deleted only when the last owning mod is uninstalled (simple per-mod lists; no reference counting in v1).
- Extraction is "relevant folders only" (no full archive dump).
- Progress + braille spinner shown.
- Errors per-mod are reported; we continue with other pending changes where possible.

This directly supports the "know exactly which files were installed so we can disable them" requirement.

## 10. Animations

Use the `rattles` crate (minimal FOSS terminal spinners, braille presets, Ratatui example, credits the unicode-animations package).

- Frames (classic braille spinner): ⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏ (cycled).
- Shown during: list/search from API, downloads, extraction, file recording/deletion, commit progress.
- Driven from the main event loop (poll with timeout for non-blocking ticks + redraw).
- Toggleable via `ui.animations` in config (default true).
- No extra heavy dependencies; fits the lightweight TUI goal.

## 11. Commit Flow Summary (UI + Mechanics)

See section 2 (UI) + section 9 (mechanics). The review screen shows the diff derived purely from comparing desired state (in DB) vs. installed paths (in DB). Apply phase is the simple direct logic above with progress + animations.

## 12. Future Enhancements (Explicitly Deferred)

- Favorites (boolean + UI filter).
- Local YAML/JSON mod definitions with overrides (currently API-only).
- Proper multi-install / multi-profile support (separate state or profiles).
- Version pinning per mod.
- Dependency graph / auto-install of deps.
- History / rollback of commits.
- More advanced extraction heuristics or per-mod post-install steps.
- Built-in update checks beyond what Forge provides.

These can be added with low risk to the core model.

## 13. Implementation Notes (High Level)

- **Language / UI**: Rust + Ratatui + crossterm.
- **Networking**: reqwest (with token in header).
- **DB**: rusqlite (simple schema as described).
- **Archives**: zip crate for .zip; consider sevenz or external for .7z if needed later (many Forge mods are zip).
- **Animations**: rattles (as chosen).
- **Config**: toml crate for config.toml.
- **Paths**: directories crate for XDG config/cache resolution.
- **Error handling**: User-friendly messages; per-mod failures during commit don't abort everything.
- **Packaging**: Already supported via flake (rustPlatform.buildRustPackage + apps entry). Binary name `spt-mod-forge`.

The basic Ratatui skeleton (fullscreen bordered "Hello World") will be replaced by the split-pane + logic described.

## 14. Visual References

All major flows have been prototyped in the live visual companion (http://localhost:64616/):
- Split-pane main view.
- Commit review + progress (with spinner).
- Config.toml example.
- Token prompt / error.
- SPT path detection.
- Cache customization.

These can be referenced during implementation for fidelity.

---

**Next Steps (per process)**: This spec is now written and approved. Self-review complete (consistent with all prior decisions, no contradictions, scope focused). User has reviewed via conversation ("looks good").

Ready to invoke writing-plans skill for implementation plan (or continue with any final clarifications). 

All major risks (state tracking, clean uninstall, CWD pollution, secret handling, API reliance) have been explicitly addressed in the design.

---

**Implementation Status Update (end of 18-task plan, post-Task 13 wiring + Tasks 16/17/18 verif+polish+self-review)**: 2026-06-05

- Full end-to-end functional "quick and easy" TUI now in worktree: split-pane (list+detail), pop-over modals (palette fuzzy command menu via :, settings live s, commit review c/y with progress), rattles braille/unicode spinners (toggleable, during loads/apply), hjkl+arrows+Space toggle everywhere, live color hot-reload + settings, cache toggle+force f, env-only FORGE_API_TOKEN (prompt C phase, hard quit on Esc if no token), SPT resolve (env>cfg>~/Games/SPTarkov + prompt, validate user/mods+BepInEx), XDG config/state.db + precedence (env>file>default), no CWD ever, simple direct install A (relevant folders only, record every file+dir in installed_files, per-mod errs continue), sqlite exact schema (managed_mods + installed_files), commit diff from DB desired vs last_installed.

- All layers wired: main.rs (terminal hygiene preserved) -> app::run_app (App coordinator, phases, event loop, on_tick apply, dispatch), ui/* (panes, palette, settings, commit renders pure), state/config/theme/forge/cache/spt/install/animation/models/error.

- 18 tasks complete (plan execution reached Task 13 full App+interactions, followed by final packaging re-verif, clippy polish, self-review+design update per "at end" + Tasks 16-18 guidance).

- Current: worktree spt-mod-forge-v1 branch at 5708f2ff85363797da6eb6fb10ce040b1ff943ec (Task 13 commit) + hygiene edits (clippy fixes) left unstaged per guidance; main tree pristine. All verif commands green (see below). DRY/YAGNI followed (thin wrappers, shared, no over-engineer), literal code no TBD/placeholders, type/name consistent, frequent prior small commits on branch only.

- Design sections: all "locked" (UI split+popovers+keys+ : +s +c +hjkl+Space, rattles, config XDG+precedence+live, token env-only+prompt, SPT ~/Games+validate+prompt, cache+toggle+force+SPT_CACHE_DIR, sqlite managed_mods/installed_files+compute_pending, simple direct A, no CWD, terminal default+3 schemes+hot-reload, per-mod continue, packaging) confirmed present in code + comments (grep evidence in plan execution log).

**Final Verification Commands Executed (all under `nix develop --command`, re-ran after fixes):**
- `cargo check` : clean (post-fix)
- `cargo test --lib` : 46 passed; 0 failed
- `cargo clippy -- -D warnings` : clean (dead_code allowed on documented unused idx in animation.rs for future; all integration lints fixed via precise edits)
- `nix build .#spt-mod-forge` : success (result -> /nix/store/...-spt-mod-forge-0.1.0 ; bin ELF present)
- `nix run .#spt-mod-forge -- --help` : non-interactive TTY error as expected (real terminal works per early tasks)
- Placeholder scan: `grep -r "TODO\|TBD\|FIXME" --include="*.rs" src/` -> 0 matches
- Git: on spt-mod-forge-v1, HEAD=5708f2f Task13, dirty only with intended hygiene (Cargo* + src edits for clippy; no new features)

Worktree spt-mod-forge-v1 branch ready for merge (or rebase) into main. See finishing guidance. 

(Design top status also updated to "Implemented".)