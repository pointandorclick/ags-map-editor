## Codebase Patterns
- Source project lives at `/Users/craig/Code/@pointandorclick/ags-map-editor/`
- This is a Tauri v2 desktop app with Rust backend and HTML/JS frontend
- `.gitignore` excludes: `node_modules/`, `src-tauri/target/`, `src-tauri/gen/schemas/`, `.DS_Store`, `**/*.rs.bk`
- `cargo check` can be used for Rust compilation verification (dependencies are now cached in `src-tauri/target/`)
- Icon file at `src-tauri/icons/icon.png` is required for `tauri::generate_context!()` to succeed
- Tauri commands use `Result<T, String>` return types for error handling via IPC
- `tauri_plugin_dialog::DialogExt` trait provides `.dialog()` on `AppHandle` for native dialogs
- `app_handle.path().app_data_dir()` provides the platform-specific app data directory
- `FilePath::into_path()` converts dialog results to `PathBuf`
- Frontend calls Tauri commands via `window.__TAURI__.core.invoke()` (requires `withGlobalTauri: true` in `tauri.conf.json`)
- `base64` crate (v0.22) uses `base64::engine::general_purpose::STANDARD.decode()` with `base64::Engine` trait import
- AGS room scripts follow the naming pattern `room{id}.asc` (no zero-padding)

## US-001: Copy source files into workspace
- Copied 9 source files from the original project to the workspace at their correct relative paths
- Files copied:
  - `src/index.html`
  - `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`
  - `src-tauri/Cargo.toml`, `src-tauri/build.rs`
  - `src-tauri/tauri.conf.json`
  - `src-tauri/capabilities/default.json`
  - `package.json`, `.gitignore`
- Removed previously-staged build artifacts (node_modules, target, gen/schemas, .DS_Store) from git index
- All file contents verified via md5 checksum comparison against source
- **Learnings for future iterations:**
  - The workspace had pre-existing staged artifacts from a previous clone; needed to `git rm --cached` them
  - The `.gitignore` is well-configured and will prevent re-adding artifacts
  - No build/lint/test tooling is set up yet (no lock file, no node_modules installed), so quality checks are not applicable at this stage

## US-002: Add Tauri dialog plugin and update capabilities
- Added `tauri-plugin-dialog = "2"` to `src-tauri/Cargo.toml` dependencies
- Added `"dialog:default"` to `src-tauri/capabilities/default.json` permissions array
- Registered dialog plugin with `.plugin(tauri_plugin_dialog::init())` in `src-tauri/src/lib.rs`
- Copied missing `src-tauri/icons/icon.png` from source project (required for `cargo check` to pass)
- `Cargo.lock` generated during dependency resolution
- Files changed:
  - `src-tauri/Cargo.toml` — added dependency
  - `src-tauri/capabilities/default.json` — added permission
  - `src-tauri/src/lib.rs` — registered plugin
  - `src-tauri/icons/icon.png` — copied from source (was missing)
  - `src-tauri/Cargo.lock` — auto-generated
  - `.gitignore` — already staged from US-001
- **Learnings for future iterations:**
  - `cargo check` works and passes — use it for Rust quality checks
  - The icon file (`src-tauri/icons/icon.png`) was missing from the US-001 copy; it's required by `tauri::generate_context!()` at compile time
  - Tauri v2 plugins follow the pattern: add crate dependency, add capability permission, register with `.plugin()` in the builder chain

## US-003: Implement Rust backend commands for project and file management
- Implemented 6 Tauri commands in `src-tauri/src/lib.rs`:
  - `pick_project_folder` — opens native directory picker, validates `Game.agf` exists, returns path
  - `validate_project` — checks `Game.agf` exists at path, returns directory basename as project name
  - `get_recent_projects` — reads `recent_projects.json` from app data dir, returns `Vec<RecentProject>`
  - `add_recent_project` — adds/moves-to-top in `recent_projects.json`, deduplicates, caps at 10
  - `load_project_data` — reads `<dirname>.agm` from project dir, returns empty string if missing
  - `save_project_data` — writes `<dirname>.agm` to project dir
- All 6 commands registered via `tauri::generate_handler![]` in the builder
- Added `RecentProject` struct with `Serialize`/`Deserialize` derives for JSON serialization
- Files changed:
  - `src-tauri/src/lib.rs` — full implementation of all 6 commands
- **Learnings for future iterations:**
  - `tauri_plugin_dialog::DialogExt` provides `.dialog()` on `AppHandle`; `blocking_pick_folder()` returns `Option<FilePath>`
  - `FilePath::into_path()` converts to `Result<PathBuf, Error>` for filesystem operations
  - `app_handle.path().app_data_dir()` (via `Manager` trait) gives platform-specific app data directory
  - `serde_json` already in `Cargo.toml` dependencies — no new crates needed for JSON operations
  - All commands use `Result<T, String>` which Tauri serializes to IPC success/error responses

## US-004: Add project selection screen overlay
- Added full-screen project selection overlay to `src/index.html` shown on startup
- Overlay includes 'AGS Map Editor' title, 'Open AGS Project...' button, and 'Recent Projects' section
- 'Open AGS Project...' button calls `pick_project_folder` Tauri command via IPC
- Recent projects loaded from `get_recent_projects` command, each showing name and path
- Clicking a recent project calls `validate_project` first; if invalid, shows error and removes from list
- After successful selection: calls `add_recent_project`, sets global `projectPath`, hides overlay, shows main UI
- Styled to match existing dark theme (#1a1a2e background, #e94560 accent for primary button)
- Toolbar and viewport hidden while overlay is visible
- Added `withGlobalTauri: true` to `tauri.conf.json` to enable `window.__TAURI__` for frontend IPC
- Files changed:
  - `src/index.html` — overlay HTML, CSS, and JavaScript
  - `src-tauri/tauri.conf.json` — added `withGlobalTauri: true`
- **Learnings for future iterations:**
  - Tauri v2 requires `"withGlobalTauri": true` in `app` section of `tauri.conf.json` for non-bundled frontends to access `window.__TAURI__`
  - `window.__TAURI__.core.invoke('command_name', { arg1, arg2 })` is the IPC call pattern
  - Tauri command errors are returned as strings in the catch block (not Error objects)
  - The `pick_project_folder` command returns the error string `"No folder selected"` when the user cancels the dialog

## US-005: Replace localStorage with file-based .agm persistence
- Replaced `loadState()` with async version that calls `invoke('load_project_data', { projectPath })` and parses returned JSON
- Replaced `saveState()` with async version that calls `invoke('save_project_data', { projectPath, data: JSON.stringify(state) })`
- Removed the `STORAGE_KEY` constant
- Eliminated all `localStorage` references from the codebase
- Updated `selectProject()` to `await loadState()` before calling `refreshMapSelect()` and `renderGrid()`
- Init flow is now async: project selection → await loadState() → refreshMapSelect() → renderGrid()
- All existing callers of `saveState()` (createMap, deleteMap, map select change, map rename, addRoom, removeRoom, updateRoom, addTemplate, mark-as-template) fire-and-forget the async call — no await needed
- Files changed:
  - `src/index.html` — replaced persistence functions and removed localStorage
- **Learnings for future iterations:**
  - Making `saveState()` async is safe for fire-and-forget callers — JavaScript will execute the async body without blocking
  - `loadState()` must be awaited at startup to ensure state is populated before rendering
  - The `projectPath` global (set in US-004's `selectProject()`) is shared by both `loadState()` and `saveState()`
  - On load failure or empty data, state is initialized to `{ maps: {}, activeMapId: null, templates: {} }` to match the default

## US-001: Add Rust backend commands for file generation I/O
- Added `base64 = "0.22"` to `src-tauri/Cargo.toml` dependencies
- Implemented 6 new Tauri commands in `src-tauri/src/lib.rs`:
  - `read_room_script` — reads `room{id}.asc` from project path, returns empty string if file missing
  - `write_room_script` — writes content to `room{id}.asc` in project path
  - `read_game_agf` — reads and returns `Game.agf` content as string
  - `write_game_agf` — creates `.bak` backup of existing `Game.agf`, then writes new content
  - `export_background_image` — decodes base64 data, ensures `Backgrounds/` directory exists, writes PNG file
  - `check_file_exists` — returns bool indicating whether file exists at given path
- All 6 commands registered in `tauri::generate_handler![]` alongside existing commands
- `cargo check` passes with no errors or warnings
- Files changed:
  - `src-tauri/Cargo.toml` — added `base64 = "0.22"` dependency
  - `src-tauri/src/lib.rs` — added `use base64::Engine;` import and 6 new command functions
  - `src-tauri/Cargo.lock` — auto-updated with base64 crate resolution
- **Learnings for future iterations:**
  - `base64` v0.22 requires importing the `Engine` trait (`use base64::Engine;`) to call `.decode()` on engine instances
  - The standard base64 engine is accessed via `base64::engine::general_purpose::STANDARD`
  - Room script files use `room{id}.asc` naming (no zero-padding) — matches AGS convention
  - `fs::copy()` is the simplest way to create backup files before overwriting
  - `fs::create_dir_all()` is safe to call even if the directory already exists
