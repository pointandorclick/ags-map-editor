use std::fs;
use std::path::Path;

use base64::Engine;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

#[derive(Serialize, Deserialize, Clone)]
struct RecentProject {
    path: String,
    name: String,
}

#[tauri::command]
async fn pick_project_folder(app_handle: AppHandle) -> Result<String, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    app_handle.dialog().file().pick_folder(move |folder| {
        let _ = tx.send(folder);
    });

    let folder = rx.recv().map_err(|e| e.to_string())?;

    match folder {
        Some(file_path) => {
            let path_buf = file_path.into_path().map_err(|e| e.to_string())?;
            let game_agf = path_buf.join("Game.agf");
            if game_agf.exists() {
                Ok(path_buf.to_string_lossy().to_string())
            } else {
                Err("Selected directory does not contain a Game.agf file".into())
            }
        }
        None => Err("No folder selected".into()),
    }
}

#[tauri::command]
fn validate_project(path: String) -> Result<String, String> {
    let project_path = Path::new(&path);
    let game_agf = project_path.join("Game.agf");
    if game_agf.exists() {
        let name = project_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("Invalid project path")?
            .to_string();
        Ok(name)
    } else {
        Err("Game.agf not found in the specified directory".into())
    }
}

#[tauri::command]
fn get_recent_projects(app_handle: AppHandle) -> Result<Vec<RecentProject>, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let recent_file = app_data_dir.join("recent_projects.json");

    if !recent_file.exists() {
        return Ok(vec![]);
    }

    let contents = fs::read_to_string(&recent_file).map_err(|e| e.to_string())?;
    let projects: Vec<RecentProject> =
        serde_json::from_str(&contents).map_err(|e| e.to_string())?;
    Ok(projects)
}

#[tauri::command]
fn add_recent_project(app_handle: AppHandle, path: String, name: String) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
    let recent_file = app_data_dir.join("recent_projects.json");

    let mut projects: Vec<RecentProject> = if recent_file.exists() {
        let contents = fs::read_to_string(&recent_file).map_err(|e| e.to_string())?;
        serde_json::from_str(&contents).unwrap_or_default()
    } else {
        vec![]
    };

    // Remove existing entry with the same path
    projects.retain(|p| p.path != path);

    // Add to the front
    projects.insert(0, RecentProject { path, name });

    // Cap at 10
    projects.truncate(10);

    let json = serde_json::to_string_pretty(&projects).map_err(|e| e.to_string())?;
    fs::write(&recent_file, json).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn load_project_data(project_path: String) -> Result<String, String> {
    let path = Path::new(&project_path);
    let dir_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid project path")?;
    let agm_file = path.join(format!("{}.agm", dir_name));

    if agm_file.exists() {
        fs::read_to_string(&agm_file).map_err(|e| e.to_string())
    } else {
        Ok(String::new())
    }
}

#[tauri::command]
fn save_project_data(project_path: String, data: String) -> Result<(), String> {
    let path = Path::new(&project_path);
    let dir_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid project path")?;
    let agm_file = path.join(format!("{}.agm", dir_name));

    fs::write(&agm_file, data).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_room_script(project_path: String, room_id: u32) -> Result<String, String> {
    let path = Path::new(&project_path).join(format!("room{}.asc", room_id));
    if path.exists() {
        fs::read_to_string(&path).map_err(|e| e.to_string())
    } else {
        Ok(String::new())
    }
}

#[tauri::command]
fn write_room_script(project_path: String, room_id: u32, content: String) -> Result<(), String> {
    let path = Path::new(&project_path).join(format!("room{}.asc", room_id));
    fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_game_agf(project_path: String) -> Result<String, String> {
    let path = Path::new(&project_path).join("Game.agf");
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_game_agf(project_path: String, content: String) -> Result<(), String> {
    let agf_path = Path::new(&project_path).join("Game.agf");
    let bak_path = Path::new(&project_path).join("Game.agf.bak");

    if agf_path.exists() {
        fs::copy(&agf_path, &bak_path).map_err(|e| e.to_string())?;
    }

    fs::write(&agf_path, content).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_background_image(
    project_path: String,
    filename: String,
    base64_data: String,
) -> Result<(), String> {
    let bg_dir = Path::new(&project_path).join("Backgrounds");
    fs::create_dir_all(&bg_dir).map_err(|e| e.to_string())?;

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&base64_data)
        .map_err(|e| e.to_string())?;

    let file_path = bg_dir.join(&filename);
    fs::write(&file_path, bytes).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_room_ids(project_path: String) -> Result<Vec<u32>, String> {
    let dir = Path::new(&project_path);
    let mut ids: Vec<u32> = fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("room") && name.ends_with(".asc") {
                name[4..name.len() - 4].parse::<u32>().ok()
            } else {
                None
            }
        })
        .collect();
    ids.sort();
    ids.dedup();
    Ok(ids)
}

#[tauri::command]
fn list_crm_files(project_path: String) -> Result<Vec<String>, String> {
    let dir = Path::new(&project_path);
    let mut files: Vec<String> = fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".crm") {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    files.sort();
    Ok(files)
}

#[tauri::command]
fn copy_crm_template(
    project_path: String,
    template_filename: String,
    room_id: u32,
) -> Result<String, String> {
    let project_dir = Path::new(&project_path);
    let source = project_dir.join(&template_filename);
    let target = project_dir.join(format!("room{}.crm", room_id));

    if target.exists() {
        return Ok("exists".to_string());
    }

    if !source.exists() {
        return Err(format!("Base room file not found: {}", template_filename));
    }

    fs::copy(&source, &target).map_err(|e| e.to_string())?;
    Ok("copied".to_string())
}

#[tauri::command]
fn check_file_exists(file_path: String) -> Result<bool, String> {
    Ok(Path::new(&file_path).exists())
}

/// Room event indices in the .crm binary format:
/// 0 = Walks off left edge,  1 = Walks off right edge,
/// 2 = Walks off bottom edge, 3 = Walks off top edge

#[derive(Deserialize)]
struct CrmEventUpdate {
    index: usize,
    handler: String,
}

/// Find the Main block (block ID 1) in a .crm file by parsing the block header structure.
/// Returns (data_start, data_length) of the Main block.
fn find_main_block(data: &[u8]) -> Result<(usize, usize), String> {
    if data.len() < 2 {
        return Err("File too small".into());
    }
    let version = u16::from_le_bytes([data[0], data[1]]);
    let use_64bit = version >= 32; // kRoomVersion_350

    let mut pos: usize = 2;
    loop {
        if pos >= data.len() {
            return Err("Unexpected end of file while scanning blocks".into());
        }
        let block_id = data[pos] as i8;
        pos += 1;

        if block_id < 0 {
            return Err("Main block not found (reached end-of-block-list)".into());
        }

        if block_id > 0 {
            let block_len = if use_64bit {
                if pos + 8 > data.len() {
                    return Err("Truncated block header".into());
                }
                let len = i64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
                pos += 8;
                len as usize
            } else {
                if pos + 4 > data.len() {
                    return Err("Truncated block header".into());
                }
                let len = i32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
                pos += 4;
                len as usize
            };

            if block_id == 1 {
                return Ok((pos, block_len));
            }
            pos += block_len;
        } else {
            // Extension block: 16-byte string ID + 8-byte length
            if pos + 24 > data.len() {
                return Err("Truncated extension block header".into());
            }
            pos += 16;
            let block_len = i64::from_le_bytes(data[pos..pos + 8].try_into().unwrap()) as usize;
            pos += 8;
            pos += block_len;
        }
    }
}

/// Read a little-endian u32 at the given offset.
fn read_u32(data: &[u8], pos: usize) -> u32 {
    u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap())
}

/// Parse null-terminated event handler strings from the data.
fn parse_event_strings(data: &[u8], start: usize, count: usize) -> Result<(Vec<String>, usize), String> {
    let mut pos = start;
    let mut events = Vec::with_capacity(count);
    for i in 0..count {
        let null_pos = data[pos..]
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| format!("Missing null terminator for event {}", i))?;
        let name = String::from_utf8_lossy(&data[pos..pos + null_pos]).to_string();
        events.push(name);
        pos = pos + null_pos + 1;
    }
    Ok((events, pos))
}

/// Validate that parsed strings look like plausible AGS event handler names.
fn looks_like_event_handlers(events: &[String]) -> bool {
    events.iter().all(|s| {
        s.is_empty()
            || s.starts_with("room_")
            || (s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && s.len() < 80)
    })
}

#[tauri::command]
fn update_crm_room_events(
    project_path: String,
    room_id: u32,
    updates: Vec<CrmEventUpdate>,
) -> Result<Vec<String>, String> {
    let crm_path = Path::new(&project_path).join(format!("room{}.crm", room_id));
    if !crm_path.exists() {
        return Err(format!("room{}.crm not found — build in AGS Editor first, then re-generate", room_id));
    }

    let data = fs::read(&crm_path).map_err(|e| e.to_string())?;

    // Find the Main block by parsing the .crm block structure
    let (main_start, main_len) = find_main_block(&data)?;
    let main_end = main_start + main_len;

    // Within the Main block, find the room event handlers.
    // The AGS format writes: [int32 legacy_vars=0] [int32 region_count] [event_handlers...]
    // Event handlers start with int32 event_count followed by null-terminated strings.
    // Search within the Main block for a valid event section.
    let mut events_count_pos = None;
    let mut event_count = 0usize;

    // Search for a 4-byte event count followed by valid null-terminated event handler strings.
    // The event count is preceded by [int32=0 (legacy vars)] [int32 region_count].
    let search_end = main_end.min(data.len());
    let mut scan = main_start;
    while scan + 12 <= search_end {
        let legacy_vars = read_u32(&data, scan);
        let region_count = read_u32(&data, scan + 4);
        let evt_count = read_u32(&data, scan + 8) as usize;

        if legacy_vars == 0 && region_count == 16 && (5..=15).contains(&evt_count) {
            // Candidate found — try parsing the event strings
            let str_start = scan + 12;
            if let Ok((events, _)) = parse_event_strings(&data, str_start, evt_count) {
                if looks_like_event_handlers(&events) {
                    events_count_pos = Some(scan + 8);
                    event_count = evt_count;
                    break;
                }
            }
        }
        scan += 1;
    }

    let events_count_pos =
        events_count_pos.ok_or_else(|| format!("Could not locate event section in room{}.crm", room_id))?;
    let events_start = events_count_pos + 4; // after the event count int32

    // Parse the event handler strings
    let (events, events_end) = parse_event_strings(&data, events_start, event_count)?;

    // Apply updates (only set handlers for empty slots to avoid overwriting user customizations)
    let mut changes = Vec::new();
    let mut new_events = events.clone();
    for update in &updates {
        if update.index >= event_count {
            continue;
        }
        if new_events[update.index] == update.handler {
            changes.push(format!(
                "{} already registered in .crm",
                update.handler
            ));
        } else if new_events[update.index].is_empty() && !update.handler.is_empty() {
            new_events[update.index] = update.handler.clone();
            changes.push(format!(
                "Registered {} in .crm",
                update.handler
            ));
        } else if !new_events[update.index].is_empty() {
            changes.push(format!(
                "Slot {} already has '{}', skipped {}",
                update.index, new_events[update.index], update.handler
            ));
        }
    }

    if new_events == events {
        return Ok(changes);
    }

    // Rebuild the event section bytes (null-terminated strings only, count stays the same)
    let mut new_section = Vec::new();
    for event in &new_events {
        new_section.extend_from_slice(event.as_bytes());
        new_section.push(0);
    }

    // Reassemble: [before events] [new event strings] [after old events]
    let mut new_data = Vec::with_capacity(data.len());
    new_data.extend_from_slice(&data[..events_start]);
    new_data.extend_from_slice(&new_section);
    new_data.extend_from_slice(&data[events_end..]);

    // Update the Main block length in the header since event strings may have changed size
    let size_diff = new_data.len() as i64 - data.len() as i64;
    if size_diff != 0 {
        let version = u16::from_le_bytes([data[0], data[1]]);
        // Block header is at offset 2: 1 byte block_id + 8 bytes length (for version >= 32)
        if version >= 32 {
            let len_offset = 3; // after version (2) + block_id (1)
            let old_len = i64::from_le_bytes(new_data[len_offset..len_offset + 8].try_into().unwrap());
            let new_len = old_len + size_diff;
            new_data[len_offset..len_offset + 8].copy_from_slice(&new_len.to_le_bytes());
        } else {
            let len_offset = 3;
            let old_len = i32::from_le_bytes(new_data[len_offset..len_offset + 4].try_into().unwrap());
            let new_len = old_len + size_diff as i32;
            new_data[len_offset..len_offset + 4].copy_from_slice(&new_len.to_le_bytes());
        }
    }

    fs::write(&crm_path, new_data).map_err(|e| e.to_string())?;

    Ok(changes)
}

#[tauri::command]
fn list_crm_room_ids(project_path: String) -> Result<Vec<u32>, String> {
    let dir = Path::new(&project_path);
    let mut ids: Vec<u32> = fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("room") && name.ends_with(".crm") {
                name[4..name.len() - 4].parse::<u32>().ok()
            } else {
                None
            }
        })
        .collect();
    ids.sort();
    ids.dedup();
    Ok(ids)
}

#[tauri::command]
fn copy_room_crm(
    project_path: String,
    template_room_id: u32,
    target_room_id: u32,
) -> Result<bool, String> {
    let dir = Path::new(&project_path);
    let target_path = dir.join(format!("room{}.crm", target_room_id));
    if target_path.exists() {
        return Ok(false);
    }
    let template_path = dir.join(format!("room{}.crm", template_room_id));
    if !template_path.exists() {
        return Err(format!(
            "Template room{}.crm not found in project",
            template_room_id
        ));
    }
    fs::copy(&template_path, &target_path).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
fn save_base_room(project_path: String, template_filename: String) -> Result<(), String> {
    let project_dir = Path::new(&project_path);
    let source = project_dir.join(&template_filename);

    if !source.exists() {
        return Err(format!("Source file not found: {}", template_filename));
    }

    let base_room_dir = project_dir.join("BaseRoom");
    fs::create_dir_all(&base_room_dir).map_err(|e| e.to_string())?;

    let target = base_room_dir.join("base-room.crm");
    fs::copy(&source, &target).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn check_background_matches(
    project_path: String,
    filename: String,
    base64_data: String,
) -> Result<bool, String> {
    let file_path = Path::new(&project_path).join("Backgrounds").join(&filename);
    if !file_path.exists() {
        return Ok(false);
    }
    let existing = fs::read(&file_path).map_err(|e| e.to_string())?;
    let new_data = base64::engine::general_purpose::STANDARD
        .decode(&base64_data)
        .map_err(|e| e.to_string())?;
    Ok(existing == new_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Create a temporary directory with a unique name for test isolation.
    fn temp_project_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ags_map_editor_test_{}_{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    // ── validate_project ───────────────────────────────────

    #[test]
    fn validate_project_ok() {
        let dir = temp_project_dir("validate_ok");
        fs::write(dir.join("Game.agf"), "<Game/>").unwrap();
        let result = validate_project(dir.to_string_lossy().to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), dir.file_name().unwrap().to_string_lossy());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_project_missing_agf() {
        let dir = temp_project_dir("validate_missing");
        let result = validate_project(dir.to_string_lossy().to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Game.agf not found"));
        let _ = fs::remove_dir_all(&dir);
    }

    // ── load_project_data / save_project_data ──────────────

    #[test]
    fn save_and_load_project_data() {
        let dir = temp_project_dir("save_load");
        let path = dir.to_string_lossy().to_string();
        let data = r#"{"maps":{}}"#.to_string();

        save_project_data(path.clone(), data.clone()).unwrap();
        let loaded = load_project_data(path).unwrap();
        assert_eq!(loaded, data);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_project_data_returns_empty_when_no_file() {
        let dir = temp_project_dir("load_empty");
        let result = load_project_data(dir.to_string_lossy().to_string()).unwrap();
        assert_eq!(result, "");
        let _ = fs::remove_dir_all(&dir);
    }

    // ── read_room_script / write_room_script ───────────────

    #[test]
    fn write_and_read_room_script() {
        let dir = temp_project_dir("room_script");
        let path = dir.to_string_lossy().to_string();
        let content = "// Room 5\nfunction room_LeaveTop() {}".to_string();

        write_room_script(path.clone(), 5, content.clone()).unwrap();
        let loaded = read_room_script(path, 5).unwrap();
        assert_eq!(loaded, content);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_room_script_returns_empty_when_missing() {
        let dir = temp_project_dir("room_missing");
        let result = read_room_script(dir.to_string_lossy().to_string(), 99).unwrap();
        assert_eq!(result, "");
        let _ = fs::remove_dir_all(&dir);
    }

    // ── read_game_agf / write_game_agf ─────────────────────

    #[test]
    fn write_game_agf_creates_backup() {
        let dir = temp_project_dir("agf_backup");
        let path = dir.to_string_lossy().to_string();
        let original = "<Game>original</Game>".to_string();
        let updated = "<Game>updated</Game>".to_string();

        // Write original
        fs::write(dir.join("Game.agf"), &original).unwrap();

        // Write via command — should create .bak
        write_game_agf(path.clone(), updated.clone()).unwrap();

        let agf_content = fs::read_to_string(dir.join("Game.agf")).unwrap();
        let bak_content = fs::read_to_string(dir.join("Game.agf.bak")).unwrap();
        assert_eq!(agf_content, updated);
        assert_eq!(bak_content, original);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_game_agf_reads_file() {
        let dir = temp_project_dir("agf_read");
        let content = "<Game><Rooms/></Game>".to_string();
        fs::write(dir.join("Game.agf"), &content).unwrap();

        let result = read_game_agf(dir.to_string_lossy().to_string()).unwrap();
        assert_eq!(result, content);
        let _ = fs::remove_dir_all(&dir);
    }

    // ── export_background_image ────────────────────────────

    #[test]
    fn export_background_image_decodes_base64() {
        let dir = temp_project_dir("bg_export");
        let path = dir.to_string_lossy().to_string();
        // Base64 for bytes [0x89, 0x50, 0x4E, 0x47] (PNG magic)
        let b64 = base64::engine::general_purpose::STANDARD.encode(&[0x89, 0x50, 0x4E, 0x47]);

        export_background_image(path, "test.png".into(), b64).unwrap();

        let saved = fs::read(dir.join("Backgrounds").join("test.png")).unwrap();
        assert_eq!(saved, vec![0x89, 0x50, 0x4E, 0x47]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_background_image_creates_backgrounds_dir() {
        let dir = temp_project_dir("bg_mkdir");
        let path = dir.to_string_lossy().to_string();
        let b64 = base64::engine::general_purpose::STANDARD.encode(b"data");

        assert!(!dir.join("Backgrounds").exists());
        export_background_image(path, "img.png".into(), b64).unwrap();
        assert!(dir.join("Backgrounds").join("img.png").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    // ── list_room_ids ──────────────────────────────────────

    #[test]
    fn list_room_ids_finds_room_files() {
        let dir = temp_project_dir("list_ids");
        fs::write(dir.join("room1.asc"), "").unwrap();
        fs::write(dir.join("room5.asc"), "").unwrap();
        fs::write(dir.join("room300.asc"), "").unwrap();
        fs::write(dir.join("other.txt"), "").unwrap();
        fs::write(dir.join("roomXY.asc"), "").unwrap(); // not a number

        let ids = list_room_ids(dir.to_string_lossy().to_string()).unwrap();
        assert_eq!(ids, vec![1, 5, 300]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_room_ids_empty_dir() {
        let dir = temp_project_dir("list_empty");
        let ids = list_room_ids(dir.to_string_lossy().to_string()).unwrap();
        assert!(ids.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    // ── check_file_exists ──────────────────────────────────

    #[test]
    fn check_file_exists_true() {
        let dir = temp_project_dir("file_exists");
        let file = dir.join("test.txt");
        fs::write(&file, "hello").unwrap();
        assert_eq!(check_file_exists(file.to_string_lossy().to_string()).unwrap(), true);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn check_file_exists_false() {
        let result = check_file_exists("/nonexistent/path/file.txt".into()).unwrap();
        assert_eq!(result, false);
    }

    // ── check_background_matches ───────────────────────────

    // ── save_base_room ───────────────────────────────────

    #[test]
    fn save_base_room_copies_file() {
        let dir = temp_project_dir("save_base_room_ok");
        let crm_data = vec![0x01, 0x02, 0x03, 0x04];
        fs::write(dir.join("room1.crm"), &crm_data).unwrap();

        save_base_room(dir.to_string_lossy().to_string(), "room1.crm".into()).unwrap();

        let saved = fs::read(dir.join("BaseRoom").join("base-room.crm")).unwrap();
        assert_eq!(saved, crm_data);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_base_room_creates_base_room_dir() {
        let dir = temp_project_dir("save_base_room_mkdir");
        fs::write(dir.join("template.crm"), b"data").unwrap();

        assert!(!dir.join("BaseRoom").exists());
        save_base_room(dir.to_string_lossy().to_string(), "template.crm".into()).unwrap();
        assert!(dir.join("BaseRoom").join("base-room.crm").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_base_room_errors_on_missing_source() {
        let dir = temp_project_dir("save_base_room_missing");
        let result = save_base_room(dir.to_string_lossy().to_string(), "nonexistent.crm".into());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Source file not found"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn check_background_matches_true() {
        let dir = temp_project_dir("bg_match");
        let bg_dir = dir.join("Backgrounds");
        fs::create_dir_all(&bg_dir).unwrap();
        let data = vec![1, 2, 3, 4];
        fs::write(bg_dir.join("room1.png"), &data).unwrap();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&data);

        let result = check_background_matches(
            dir.to_string_lossy().to_string(),
            "room1.png".into(),
            b64,
        ).unwrap();
        assert!(result);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn check_background_matches_false_different_data() {
        let dir = temp_project_dir("bg_diff");
        let bg_dir = dir.join("Backgrounds");
        fs::create_dir_all(&bg_dir).unwrap();
        fs::write(bg_dir.join("room1.png"), &[1, 2, 3]).unwrap();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&[9, 9, 9]);

        let result = check_background_matches(
            dir.to_string_lossy().to_string(),
            "room1.png".into(),
            b64,
        ).unwrap();
        assert!(!result);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn check_background_matches_false_missing_file() {
        let dir = temp_project_dir("bg_missing");
        let b64 = base64::engine::general_purpose::STANDARD.encode(b"data");

        let result = check_background_matches(
            dir.to_string_lossy().to_string(),
            "nope.png".into(),
            b64,
        ).unwrap();
        assert!(!result);
        let _ = fs::remove_dir_all(&dir);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            pick_project_folder,
            validate_project,
            get_recent_projects,
            add_recent_project,
            load_project_data,
            save_project_data,
            read_room_script,
            write_room_script,
            read_game_agf,
            write_game_agf,
            export_background_image,
            list_room_ids,
            list_crm_files,
            copy_crm_template,
            check_file_exists,
            check_background_matches,
            update_crm_room_events,
            list_crm_room_ids,
            copy_room_crm,
            save_base_room,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
