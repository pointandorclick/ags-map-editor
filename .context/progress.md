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
