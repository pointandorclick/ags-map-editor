## Codebase Patterns
- Source project lives at `/Users/craig/Code/@pointandorclick/ags-map-editor/`
- This is a Tauri v2 desktop app with Rust backend and HTML/JS frontend
- `.gitignore` excludes: `node_modules/`, `src-tauri/target/`, `src-tauri/gen/schemas/`, `.DS_Store`, `**/*.rs.bk`

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
