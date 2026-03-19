use std::fs;
use std::path::Path;
use std::time::Duration;

use base64::Engine;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

/// Validate that a filename does not contain path traversal sequences or separators.
fn validate_filename(name: &str) -> Result<(), String> {
    if name.contains("..") || name.contains('/') || name.contains('\\') || name.is_empty() {
        return Err(format!("Invalid filename: {}", name));
    }
    Ok(())
}

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

    let folder = rx.recv_timeout(Duration::from_secs(600))
        .map_err(|_| "Folder selection timed out".to_string())?;

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
        serde_json::from_str(&contents).unwrap_or_else(|_| {
            eprintln!("Warning: recent_projects.json was corrupted, resetting list");
            vec![]
        })
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
    validate_filename(&filename)?;
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
fn copy_base_room_file(
    project_path: String,
    room_id: u32,
) -> Result<String, String> {
    let project_dir = Path::new(&project_path);
    let source = project_dir.join("BaseRoom").join("base-room.crm");
    let target = project_dir.join(format!("room{}.crm", room_id));

    if target.exists() {
        return Ok("exists".to_string());
    }

    if !source.exists() {
        return Err("Base room file not found: BaseRoom/base-room.crm".to_string());
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
fn read_u32(data: &[u8], pos: usize) -> Result<u32, String> {
    let bytes: [u8; 4] = data.get(pos..pos + 4)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| format!("Truncated data at offset {}", pos))?;
    Ok(u32::from_le_bytes(bytes))
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
        let legacy_vars = read_u32(&data, scan)?;
        let region_count = read_u32(&data, scan + 4)?;
        let evt_count = read_u32(&data, scan + 8)? as usize;

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
            let new_len = old_len as i64 + size_diff;
            let new_len_i32 = i32::try_from(new_len)
                .map_err(|_| format!("Block length {} overflows i32", new_len))?;
            new_data[len_offset..len_offset + 4].copy_from_slice(&new_len_i32.to_le_bytes());
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
    validate_filename(&template_filename)?;
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
fn copy_all_room_files(
    project_path: String,
    source_room_id: u32,
    target_room_id: u32,
    force: bool,
) -> Result<Vec<String>, String> {
    let dir = Path::new(&project_path);
    let prefix = format!("room{}.", source_room_id);
    let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
    let mut results: Vec<String> = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with(&prefix) {
            continue;
        }
        let ext = &name[prefix.len() - 1..]; // includes the dot, e.g. ".crm"
        let target_name = format!("room{}{}", target_room_id, ext);
        let target_path = dir.join(&target_name);

        if target_path.exists() && !force {
            results.push(format!("skipped:{}", target_name));
        } else {
            fs::copy(entry.path(), &target_path).map_err(|e| e.to_string())?;
            results.push(format!("copied:{}", target_name));
        }
    }

    results.sort();
    Ok(results)
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

// ══════════════════════════════════════════════════════════════════════
//  LZSS compression (AGS "lzw" format — actually LZSS)
// ══════════════════════════════════════════════════════════════════════

const LZSS_N: usize = 4096; // Ring buffer / sliding window size
const LZSS_F: usize = 16; // Lookahead buffer size / max match length
const LZSS_THRESHOLD: usize = 3; // Minimum match length to encode as reference
/// Decompress LZSS data from `src` into a buffer of `dst_sz` bytes.
/// Returns the decompressed bytes.
fn lzss_expand(src: &[u8], dst_sz: usize) -> Result<Vec<u8>, String> {
    if dst_sz == 0 {
        return Err("lzss_expand: zero destination size".into());
    }
    let mut ring = vec![0u8; LZSS_N];
    let mut dst = Vec::with_capacity(dst_sz);
    let mut si = 0usize; // source index
    let mut ri = LZSS_N - LZSS_F; // ring buffer write position

    while si < src.len() && dst.len() < dst_sz {
        let bits = src[si] as u32;
        si += 1;
        let mut mask = 0x01u32;
        while (mask & 0xFF) != 0 {
            if dst.len() >= dst_sz || si >= src.len() {
                break;
            }
            if bits & mask != 0 {
                // Match reference: 2-byte LE int16
                if si + 2 > src.len() {
                    break;
                }
                let j = i16::from_le_bytes([src[si], src[si + 1]]);
                si += 2;
                let len = (((j >> 12) & 0x0F) as usize) + LZSS_THRESHOLD;
                let mut offset = ((ri as i32 - j as i32 - 1) & (LZSS_N as i32 - 1)) as usize;
                if dst.len() + len > dst_sz {
                    break;
                }
                for _ in 0..len {
                    let ch = ring[offset];
                    dst.push(ch);
                    ring[ri] = ch;
                    offset = (offset + 1) & (LZSS_N - 1);
                    ri = (ri + 1) & (LZSS_N - 1);
                }
            } else {
                // Literal byte
                let ch = src[si];
                si += 1;
                dst.push(ch);
                ring[ri] = ch;
                ri = (ri + 1) & (LZSS_N - 1);
            }
            mask <<= 1;
        }
    }
    Ok(dst)
}

/// Compress `data` using LZSS. Returns the compressed bytes.
/// Uses a hash-chain for fast match finding. Matches are limited to positions
/// already output by the encoder (so the AGS decompressor can reproduce them
/// regardless of ring-buffer initialisation).
fn lzss_compress(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }

    const HASH_SIZE: usize = 4096;
    const HASH_MASK: usize = HASH_SIZE - 1;
    const MAX_CHAIN: usize = 128;
    let mut hash_head = [usize::MAX; HASH_SIZE];
    let mut hash_prev = vec![usize::MAX; LZSS_N];

    #[inline(always)]
    fn hash3(a: u8, b: u8, c: u8) -> usize {
        ((a as usize) << 8 ^ (b as usize) << 4 ^ (c as usize)) & HASH_MASK
    }

    let mut ring = vec![0u8; LZSS_N];
    let mut out = Vec::with_capacity(data.len());

    let mut code_buf = [0u8; 17];
    let mut code_buf_ptr = 1usize;
    let mut mask = 1u8;
    code_buf[0] = 0;

    let initial_fill = LZSS_F.min(data.len());
    let mut win_pos = LZSS_N - LZSS_F;
    for k in 0..initial_fill {
        ring[(win_pos + k) & (LZSS_N - 1)] = data[k];
    }
    let mut di = initial_fill;
    let total_len = data.len();
    let mut encoded = 0usize;

    // Monotonic counter of ring writes, used to reject stale hash entries
    // that point to ring positions overwritten since insertion.
    let mut write_seq = vec![0u64; LZSS_N]; // sequence number when position was last written
    let mut insert_seq = vec![0u64; LZSS_N]; // sequence number when position was inserted
    let mut seq: u64 = 1;

    // Mark the initial fill positions as written
    for k in 0..initial_fill {
        let rp = (win_pos + k) & (LZSS_N - 1);
        write_seq[rp] = seq;
    }

    while encoded < total_len {
        let remaining = total_len - encoded;
        let max_match = LZSS_F.min(remaining);
        let mut best_len = 0usize;
        let mut best_offset = 0usize;
        let wp = win_pos & (LZSS_N - 1);

        if max_match >= LZSS_THRESHOLD && encoded >= LZSS_THRESHOLD {
            let h = hash3(
                ring[wp],
                ring[(wp + 1) & (LZSS_N - 1)],
                ring[(wp + 2) & (LZSS_N - 1)],
            );
            let mut candidate = hash_head[h];
            let mut chain_len = 0;

            while candidate != usize::MAX && chain_len < MAX_CHAIN {
                // Reject stale entries: if position was rewritten after insertion
                if candidate != wp && insert_seq[candidate] >= write_seq[candidate] {
                    let dist = (wp + LZSS_N - candidate) & (LZSS_N - 1);
                    if dist > 0 && dist <= encoded.min(LZSS_N - 1) {
                        let mut mlen = 0;
                        while mlen < max_match
                            && ring[(candidate + mlen) & (LZSS_N - 1)]
                                == ring[(wp + mlen) & (LZSS_N - 1)]
                        {
                            mlen += 1;
                        }
                        if mlen > best_len {
                            best_len = mlen;
                            best_offset = dist;
                            if best_len >= LZSS_F {
                                break;
                            }
                        }
                    }
                }
                candidate = hash_prev[candidate];
                chain_len += 1;
            }
        }

        if best_len >= LZSS_THRESHOLD {
            code_buf[0] |= mask;
            let encoded_val =
                (((best_len - LZSS_THRESHOLD) as u16) << 12) | ((best_offset - 1) as u16 & 0xFFF);
            let bytes = (encoded_val as i16).to_le_bytes();
            code_buf[code_buf_ptr] = bytes[0];
            code_buf[code_buf_ptr + 1] = bytes[1];
            code_buf_ptr += 2;

            for _ in 0..best_len {
                // Insert into hash chain
                let rp = win_pos & (LZSS_N - 1);
                let a = ring[rp];
                let b = ring[(rp + 1) & (LZSS_N - 1)];
                let c = ring[(rp + 2) & (LZSS_N - 1)];
                let h = hash3(a, b, c);
                hash_prev[rp] = hash_head[h];
                hash_head[h] = rp;
                insert_seq[rp] = seq;

                // Load next byte from input
                let load_pos = (win_pos + LZSS_F) & (LZSS_N - 1);
                if di < total_len {
                    ring[load_pos] = data[di];
                    di += 1;
                }
                seq += 1;
                write_seq[load_pos] = seq;

                win_pos = (win_pos + 1) & (LZSS_N - 1);
            }
            encoded += best_len;
        } else {
            code_buf[code_buf_ptr] = ring[wp];
            code_buf_ptr += 1;

            // Insert into hash chain
            let a = ring[wp];
            let b = ring[(wp + 1) & (LZSS_N - 1)];
            let c = ring[(wp + 2) & (LZSS_N - 1)];
            let h = hash3(a, b, c);
            hash_prev[wp] = hash_head[h];
            hash_head[h] = wp;
            insert_seq[wp] = seq;

            let load_pos = (win_pos + LZSS_F) & (LZSS_N - 1);
            if di < total_len {
                ring[load_pos] = data[di];
                di += 1;
            }
            seq += 1;
            write_seq[load_pos] = seq;

            win_pos = (win_pos + 1) & (LZSS_N - 1);
            encoded += 1;
        }

        mask <<= 1;
        if mask == 0 {
            out.extend_from_slice(&code_buf[..code_buf_ptr]);
            code_buf_ptr = 1;
            mask = 1;
            code_buf[0] = 0;
        }
    }

    if code_buf_ptr > 1 {
        out.extend_from_slice(&code_buf[..code_buf_ptr]);
    }

    out
}

// ══════════════════════════════════════════════════════════════════════
//  .crm Main Block parser — locates background image offset
// ══════════════════════════════════════════════════════════════════════

/// Read a little-endian i16 at the given offset.
fn read_i16(data: &[u8], pos: usize) -> i16 {
    i16::from_le_bytes(data[pos..pos + 2].try_into().unwrap())
}

/// Read a little-endian i32 at the given offset.
fn read_i32(data: &[u8], pos: usize) -> i32 {
    i32::from_le_bytes(data[pos..pos + 4].try_into().unwrap())
}

/// Information about the background image in a .crm Main block.
struct CrmBackgroundInfo {
    /// Byte offset in the file where the background data begins (the 1024-byte palette).
    offset: usize,
    /// Total size of the background blob: 1024 (palette) + 4 (uncomp) + 4 (comp) + comp_size.
    total_size: usize,
    /// Bytes per pixel of the background (1, 2, or 4).
    bpp: u32,
    /// Background image width in pixels.
    width: u32,
    /// Background image height in pixels.
    height: u32,
}

/// Skip a length-prefixed string (int32 len + len bytes) and return new position.
fn skip_length_prefixed_string(data: &[u8], pos: usize) -> Result<usize, String> {
    if pos + 4 > data.len() {
        return Err("Truncated length-prefixed string".into());
    }
    let len = read_i32(data, pos) as usize;
    let end = pos + 4 + len;
    if end > data.len() {
        return Err("Length-prefixed string extends past data".into());
    }
    Ok(end)
}

/// Skip a null-terminated string and return position after the null byte.
fn skip_null_terminated_string(data: &[u8], pos: usize) -> Result<usize, String> {
    let null_pos = data[pos..]
        .iter()
        .position(|&b| b == 0)
        .ok_or("Missing null terminator")?;
    Ok(pos + null_pos + 1)
}

/// Parse the Main block of a .crm file to locate the background image.
/// `data` is the full .crm file bytes, `main_start`/`main_len` from find_main_block().
fn find_background_in_main(
    data: &[u8],
    main_start: usize,
    main_len: usize,
    version: u16,
) -> Result<CrmBackgroundInfo, String> {
    let main_end = main_start + main_len;
    let mut pos = main_start;

    // Helper to check bounds
    macro_rules! need {
        ($n:expr) => {
            if pos + $n > main_end {
                return Err(format!(
                    "Main block truncated at offset {} (need {} more bytes, {} remain)",
                    pos, $n, main_end.saturating_sub(pos)
                ));
            }
        };
    }

    // 1. BackgroundBPP (int32)
    need!(4);
    let bpp = read_i32(data, pos) as u32;
    pos += 4;

    // 2. WalkBehindCount (int16)
    need!(2);
    let walk_behind_count = read_i16(data, pos) as usize;
    pos += 2;

    // 3. Walk-behind baselines: int16 × count
    need!(walk_behind_count * 2);
    pos += walk_behind_count * 2;

    // 4. HotspotCount (int32)
    need!(4);
    let hotspot_count = read_i32(data, pos) as usize;
    pos += 4;

    // 5. Hotspot walk-to points: (int16 x, int16 y) × count
    need!(hotspot_count * 4);
    pos += hotspot_count * 4;

    // 6. Hotspot names — version >= 31 uses length-prefixed, 28-30 uses null-terminated
    for _ in 0..hotspot_count {
        if version >= 31 {
            pos = skip_length_prefixed_string(data, pos)?;
        } else if version >= 28 {
            pos = skip_null_terminated_string(data, pos)?;
        } else {
            // Fixed 30-byte names
            need!(30);
            pos += 30;
        }
    }

    // 7. Hotspot script names — version >= 31 uses length-prefixed, else fixed 20 bytes
    if version >= 24 {
        for _ in 0..hotspot_count {
            if version >= 31 {
                pos = skip_length_prefixed_string(data, pos)?;
            } else {
                need!(20);
                pos += 20; // LEGACY_MAX_SCRIPT_NAME_LEN = 20
            }
        }
    }

    // 8. Legacy poly-point areas: int32 count (always 0)
    need!(4);
    let poly_count = read_i32(data, pos) as usize;
    pos += 4;
    if poly_count != 0 {
        return Err(format!("Unexpected poly-point areas count: {}", poly_count));
    }

    // 9. Room edges: 4 × int16
    need!(8);
    pos += 8;

    // 10. Object count (int16) + objects (10 bytes each)
    need!(2);
    let obj_count = read_i16(data, pos) as usize;
    pos += 2;
    need!(obj_count * 10);
    pos += obj_count * 10;

    // 11. Local variables: int32 count + variable data
    if version >= 24 {
        need!(4);
        let localvar_count = read_i32(data, pos) as usize;
        pos += 4;
        // Each InteractionVariable: null-terminated name + int8 type + int32 value
        for _ in 0..localvar_count {
            pos = skip_null_terminated_string(data, pos)?;
            need!(5);
            pos += 5;
        }
    }

    // 12. Region count (int32) — version >= 21
    let region_count = if version >= 21 {
        need!(4);
        let rc = read_i32(data, pos) as usize;
        pos += 4;
        rc
    } else {
        0
    };

    // 13. Event handler section (version >= 26):
    //     Room events + hotspot_count groups + obj_count groups + region_count groups
    //     Each group: int32 count + count × null-terminated strings
    if version >= 26 {
        let groups = 1 + hotspot_count + obj_count + region_count;
        for _ in 0..groups {
            need!(4);
            let evt_count = read_i32(data, pos) as usize;
            pos += 4;
            for _ in 0..evt_count {
                pos = skip_null_terminated_string(data, pos)?;
            }
        }
    }

    // 14. Object baselines: int32 × obj_count
    need!(obj_count * 4);
    pos += obj_count * 4;

    // 15. Room dimensions: int16 width, int16 height
    need!(4);
    let room_width = read_i16(data, pos) as u32;
    let _room_height = read_i16(data, pos + 2) as u32;
    pos += 4;

    // 16. Object flags: int16 × obj_count
    need!(obj_count * 2);
    pos += obj_count * 2;

    // 17. MaskResolution (int16)
    need!(2);
    pos += 2;

    // 18. Walk area data: int32 count + 5 arrays of int16 × count
    need!(4);
    let walk_area_count = read_i32(data, pos) as usize;
    pos += 4;
    need!(walk_area_count * 2 * 5);
    pos += walk_area_count * 2 * 5; // ScalingFar, PlayerView, ScalingNear, Top, Bottom

    // 19. Password (11 bytes)
    need!(11);
    pos += 11;

    // 20. Room options: 6 bytes + 4 reserved = 10 bytes
    need!(10);
    pos += 10;

    // 21. MessageCount (int16)
    need!(2);
    let msg_count = read_i16(data, pos) as usize;
    pos += 2;

    // 22. GameID (int32) — version >= 25
    if version >= 25 {
        need!(4);
        pos += 4;
    }

    // 23. Message info: 2 bytes each (DisplayAs + Flags)
    need!(msg_count * 2);
    pos += msg_count * 2;

    // 24. Messages: encrypted length-prefixed strings (int32 len + len bytes)
    for _ in 0..msg_count {
        pos = skip_length_prefixed_string(data, pos)?;
    }

    // 25. Legacy animation count (int16 = 0)
    need!(2);
    pos += 2;

    // 26. Walk area PlayerView duplicate: MAX_WALK_AREAS (16) × int16
    need!(16 * 2);
    pos += 16 * 2;

    // 27. Region light levels: region_count × int16
    need!(region_count * 2);
    pos += region_count * 2;

    // 28. Region tints: region_count × int32
    need!(region_count * 4);
    pos += region_count * 4;

    // ── Background image starts here ──
    let bg_offset = pos;

    // Parse the background header to get its total size:
    // 1024 bytes palette + int32 uncomp_size + int32 comp_size + comp_size bytes
    need!(1024 + 4 + 4);
    pos += 1024; // palette
    let _uncomp_size = read_i32(data, pos) as usize;
    pos += 4;
    let comp_size = read_i32(data, pos) as usize;
    pos += 4;
    need!(comp_size);

    let bg_total_size = 1024 + 4 + 4 + comp_size;

    // Decode the background to get actual width/height
    let uncomp = lzss_expand(&data[pos..pos + comp_size], _uncomp_size)?;
    let stride = i32::from_le_bytes(uncomp[0..4].try_into().unwrap()) as u32;
    let height = i32::from_le_bytes(uncomp[4..8].try_into().unwrap()) as u32;
    let width = if bpp > 0 { stride / bpp } else { room_width };

    Ok(CrmBackgroundInfo {
        offset: bg_offset,
        total_size: bg_total_size,
        bpp,
        width,
        height,
    })
}

/// Encode raw pixel data as an AGS background blob:
/// 1024-byte palette + int32 uncomp_size + int32 comp_size + LZSS-compressed data.
fn encode_background(pixels: &[u8], width: u32, height: u32, bpp: u32) -> Vec<u8> {
    let stride = width * bpp;

    // Build the uncompressed inner buffer: int32 stride + int32 height + pixel data
    let mut inner = Vec::with_capacity(8 + pixels.len());
    inner.extend_from_slice(&(stride as i32).to_le_bytes());
    inner.extend_from_slice(&(height as i32).to_le_bytes());
    inner.extend_from_slice(pixels);

    let uncomp_size = inner.len() as i32;

    // LZSS compress the inner buffer
    let compressed = lzss_compress(&inner);
    let comp_size = compressed.len() as i32;

    // Assemble the full background blob
    let mut blob = Vec::with_capacity(1024 + 4 + 4 + compressed.len());
    // 1024-byte palette (zeroed for 32-bit / non-8-bit images)
    blob.extend_from_slice(&[0u8; 1024]);
    blob.extend_from_slice(&uncomp_size.to_le_bytes());
    blob.extend_from_slice(&comp_size.to_le_bytes());
    blob.extend_from_slice(&compressed);

    blob
}

/// Convert RGBA pixels (from the `image` crate) to AGS BGRA format for 32-bit backgrounds.
fn rgba_to_bgra(rgba: &[u8]) -> Vec<u8> {
    let mut bgra = Vec::with_capacity(rgba.len());
    for chunk in rgba.chunks_exact(4) {
        bgra.push(chunk[2]); // B
        bgra.push(chunk[1]); // G
        bgra.push(chunk[0]); // R
        bgra.push(chunk[3]); // A
    }
    bgra
}

#[tauri::command]
fn embed_image_in_crm(
    project_path: String,
    room_id: u32,
    base64_image_data: String,
) -> Result<String, String> {
    let crm_path = Path::new(&project_path).join(format!("room{}.crm", room_id));
    if !crm_path.exists() {
        return Err(format!("room{}.crm not found", room_id));
    }

    let data = fs::read(&crm_path).map_err(|e| e.to_string())?;

    if data.len() < 3 {
        return Err(format!(
            "room{}.crm is too small ({} bytes) — it may be corrupt. \
             Build the room in AGS Editor first, then re-generate.",
            room_id,
            data.len()
        ));
    }

    // Decode the user's image from base64 PNG/JPEG
    let image_bytes = base64::engine::general_purpose::STANDARD
        .decode(&base64_image_data)
        .map_err(|e| format!("Failed to decode base64 image: {}", e))?;

    let img = image::load_from_memory(&image_bytes)
        .map_err(|e| format!("Failed to decode image: {}", e))?;
    let img_width = img.width();
    let img_height = img.height();

    // Parse the .crm to find the background
    let version = u16::from_le_bytes([data[0], data[1]]);
    let (main_start, main_len) = find_main_block(&data)?;

    let bg_info = find_background_in_main(&data, main_start, main_len, version)
        .map_err(|e| format!("Failed to parse .crm main block: {}", e))?;

    // Dimension check
    if img_width != bg_info.width || img_height != bg_info.height {
        return Err(format!(
            "Image dimensions {}x{} do not match room background {}x{}. \
             Resize the image to match the base room before embedding.",
            img_width, img_height, bg_info.width, bg_info.height
        ));
    }

    // Convert image to the correct pixel format
    let pixel_data = match bg_info.bpp {
        4 => {
            let rgba = img.to_rgba8();
            rgba_to_bgra(rgba.as_raw())
        }
        2 => {
            // 16-bit: R5G6B5 little-endian
            let rgb = img.to_rgb8();
            let mut out = Vec::with_capacity((img_width * img_height * 2) as usize);
            for pixel in rgb.pixels() {
                let r = (pixel[0] as u16 >> 3) & 0x1F;
                let g = (pixel[1] as u16 >> 2) & 0x3F;
                let b = (pixel[2] as u16 >> 3) & 0x1F;
                let val = (r << 11) | (g << 5) | b;
                out.extend_from_slice(&val.to_le_bytes());
            }
            out
        }
        1 => {
            return Err("8-bit (256 color) room backgrounds are not supported for image embedding. Use a 32-bit or 16-bit base room.".into());
        }
        _ => {
            return Err(format!("Unsupported background BPP: {}", bg_info.bpp));
        }
    };

    // Encode the new background
    let new_bg = encode_background(&pixel_data, img_width, img_height, bg_info.bpp);

    // Splice into the .crm file: replace old background with new
    let bg_end = bg_info.offset + bg_info.total_size;
    let mut new_data = Vec::with_capacity(data.len() - bg_info.total_size + new_bg.len());
    new_data.extend_from_slice(&data[..bg_info.offset]);
    new_data.extend_from_slice(&new_bg);
    new_data.extend_from_slice(&data[bg_end..]);

    // Update the Main block length in the header
    let size_diff = new_data.len() as i64 - data.len() as i64;
    if size_diff != 0 {
        if version >= 32 {
            // 64-bit block length at offset 3 (after 2-byte version + 1-byte block_id)
            let len_offset = 3;
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

    Ok(format!(
        "Embedded {}x{} background in room{}.crm",
        img_width, img_height, room_id
    ))
}

#[tauri::command]
fn delete_background_image(project_path: String, filename: String) -> Result<bool, String> {
    let file_path = Path::new(&project_path).join("Backgrounds").join(&filename);
    if file_path.exists() {
        fs::remove_file(&file_path).map_err(|e| e.to_string())?;
        Ok(true)
    } else {
        Ok(false)
    }
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

    // ── copy_all_room_files ───────────────────────────────────

    #[test]
    fn copy_all_room_files_basic_copy() {
        let dir = temp_project_dir("copy_all_basic");
        let path = dir.to_string_lossy().to_string();
        fs::write(dir.join("room5.crm"), b"crm-data").unwrap();
        fs::write(dir.join("room5.asc"), b"script-data").unwrap();

        let result = copy_all_room_files(path, 5, 10, false).unwrap();
        assert_eq!(result, vec!["copied:room10.asc", "copied:room10.crm"]);
        assert_eq!(fs::read_to_string(dir.join("room10.crm")).unwrap(), "crm-data");
        assert_eq!(fs::read_to_string(dir.join("room10.asc")).unwrap(), "script-data");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn copy_all_room_files_force_overwrite() {
        let dir = temp_project_dir("copy_all_force");
        let path = dir.to_string_lossy().to_string();
        fs::write(dir.join("room1.crm"), b"new-data").unwrap();
        fs::write(dir.join("room2.crm"), b"old-data").unwrap();

        let result = copy_all_room_files(path, 1, 2, true).unwrap();
        assert_eq!(result, vec!["copied:room2.crm"]);
        assert_eq!(fs::read_to_string(dir.join("room2.crm")).unwrap(), "new-data");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn copy_all_room_files_skip_existing() {
        let dir = temp_project_dir("copy_all_skip");
        let path = dir.to_string_lossy().to_string();
        fs::write(dir.join("room1.crm"), b"source").unwrap();
        fs::write(dir.join("room2.crm"), b"existing").unwrap();

        let result = copy_all_room_files(path, 1, 2, false).unwrap();
        assert_eq!(result, vec!["skipped:room2.crm"]);
        assert_eq!(fs::read_to_string(dir.join("room2.crm")).unwrap(), "existing");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn copy_all_room_files_no_source_files() {
        let dir = temp_project_dir("copy_all_none");
        let path = dir.to_string_lossy().to_string();
        fs::write(dir.join("room99.crm"), b"other").unwrap();

        let result = copy_all_room_files(path, 1, 2, false).unwrap();
        assert!(result.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    // ── validate_filename ────────────────────────────────

    #[test]
    fn validate_filename_rejects_traversal() {
        assert!(validate_filename("../etc/passwd").is_err());
        assert!(validate_filename("..\\windows\\system32").is_err());
        assert!(validate_filename("foo/bar.crm").is_err());
        assert!(validate_filename("foo\\bar.crm").is_err());
        assert!(validate_filename("").is_err());
    }

    #[test]
    fn validate_filename_accepts_normal_names() {
        assert!(validate_filename("room1.crm").is_ok());
        assert!(validate_filename("test.png").is_ok());
        assert!(validate_filename("my-file_v2.txt").is_ok());
    }

    #[test]
    fn export_background_rejects_path_traversal() {
        let dir = temp_project_dir("bg_traversal");
        let path = dir.to_string_lossy().to_string();
        let b64 = base64::engine::general_purpose::STANDARD.encode(b"data");

        let result = export_background_image(path, "../evil.png".into(), b64);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid filename"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_base_room_rejects_path_traversal() {
        let dir = temp_project_dir("base_traversal");
        let result = save_base_room(dir.to_string_lossy().to_string(), "../evil.crm".into());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid filename"));
        let _ = fs::remove_dir_all(&dir);
    }

    // ── LZSS compression tests ────────────────────────────

    #[test]
    fn lzss_roundtrip_simple() {
        let data = b"Hello, World! Hello, World! Hello, World!";
        let compressed = lzss_compress(data);
        let decompressed = lzss_expand(&compressed, data.len()).unwrap();
        assert_eq!(&decompressed, data);
    }

    #[test]
    fn lzss_roundtrip_all_zeros() {
        let data = vec![0u8; 1024];
        let compressed = lzss_compress(&data);
        assert!(compressed.len() < data.len(), "should compress repeated data");
        let decompressed = lzss_expand(&compressed, data.len()).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn lzss_roundtrip_random_like() {
        // Data with no obvious patterns — should still round-trip
        let data: Vec<u8> = (0..500).map(|i| ((i * 97 + 31) % 256) as u8).collect();
        let compressed = lzss_compress(&data);
        let decompressed = lzss_expand(&compressed, data.len()).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn lzss_roundtrip_single_byte() {
        let data = b"X";
        let compressed = lzss_compress(data);
        let decompressed = lzss_expand(&compressed, data.len()).unwrap();
        assert_eq!(&decompressed, data);
    }

    #[test]
    fn lzss_roundtrip_pixel_data() {
        // Simulates a small AGS background: stride+height header + pixel data
        let width: u32 = 16;
        let height: u32 = 16;
        let bpp: u32 = 4;
        let stride = width * bpp;
        let mut data = Vec::new();
        data.extend_from_slice(&(stride as i32).to_le_bytes());
        data.extend_from_slice(&(height as i32).to_le_bytes());
        // Add pixel data (gradient pattern)
        for y in 0..height {
            for x in 0..width {
                data.push((x * 16) as u8); // B
                data.push((y * 16) as u8); // G
                data.push(0);              // R
                data.push(255);            // A
            }
        }
        let compressed = lzss_compress(&data);
        let decompressed = lzss_expand(&compressed, data.len()).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn lzss_roundtrip_large_image() {
        // Simulates a realistic 320x200 32-bit background (~256KB)
        let width = 320u32;
        let height = 200u32;
        let bpp = 4u32;
        let stride = width * bpp;
        let mut data = Vec::with_capacity((8 + stride * height) as usize);
        data.extend_from_slice(&(stride as i32).to_le_bytes());
        data.extend_from_slice(&(height as i32).to_le_bytes());
        for y in 0..height {
            for x in 0..width {
                data.push((x & 0xFF) as u8);
                data.push((y & 0xFF) as u8);
                data.push(((x + y) & 0xFF) as u8);
                data.push(255);
            }
        }
        let compressed = lzss_compress(&data);
        let decompressed = lzss_expand(&compressed, data.len()).unwrap();
        assert_eq!(decompressed, data);
    }

    /// Decompress with a garbage-filled ring buffer (simulates AGS's malloc without memset).
    /// If our compressor ever references ring positions not previously written by the
    /// decompressor, this test will catch it.
    fn lzss_expand_dirty_ring(src: &[u8], dst_sz: usize) -> Result<Vec<u8>, String> {
        if dst_sz == 0 {
            return Err("zero dst".into());
        }
        // Fill ring with 0xAA (garbage) instead of zeros
        let mut ring = vec![0xAAu8; LZSS_N];
        let mut dst = Vec::with_capacity(dst_sz);
        let mut si = 0usize;
        let mut ri = LZSS_N - LZSS_F;
        while si < src.len() && dst.len() < dst_sz {
            let bits = src[si] as u32;
            si += 1;
            let mut mask = 0x01u32;
            while (mask & 0xFF) != 0 {
                if dst.len() >= dst_sz || si >= src.len() { break; }
                if bits & mask != 0 {
                    if si + 2 > src.len() { break; }
                    let j = i16::from_le_bytes([src[si], src[si + 1]]);
                    si += 2;
                    let len = (((j >> 12) & 0x0F) as usize) + LZSS_THRESHOLD;
                    let mut offset = ((ri as i32 - j as i32 - 1) & (LZSS_N as i32 - 1)) as usize;
                    if dst.len() + len > dst_sz { break; }
                    for _ in 0..len {
                        let ch = ring[offset];
                        dst.push(ch);
                        ring[ri] = ch;
                        offset = (offset + 1) & (LZSS_N - 1);
                        ri = (ri + 1) & (LZSS_N - 1);
                    }
                } else {
                    let ch = src[si];
                    si += 1;
                    dst.push(ch);
                    ring[ri] = ch;
                    ri = (ri + 1) & (LZSS_N - 1);
                }
                mask <<= 1;
            }
        }
        Ok(dst)
    }

    #[test]
    fn lzss_dirty_ring_roundtrip_large_image() {
        // A realistic image that would expose ring-init-dependent corruption
        let width = 320u32;
        let height = 200u32;
        let bpp = 4u32;
        let stride = width * bpp;
        let mut data = Vec::with_capacity((8 + stride * height) as usize);
        data.extend_from_slice(&(stride as i32).to_le_bytes());
        data.extend_from_slice(&(height as i32).to_le_bytes());
        for y in 0..height {
            for x in 0..width {
                data.push((x & 0xFF) as u8);
                data.push((y & 0xFF) as u8);
                data.push(((x + y) & 0xFF) as u8);
                data.push(255);
            }
        }
        let compressed = lzss_compress(&data);
        // Decompress with dirty ring — must still produce correct output
        let decompressed = lzss_expand_dirty_ring(&compressed, data.len()).unwrap();
        assert_eq!(decompressed.len(), data.len());
        // Find first mismatch if any
        for (i, (a, b)) in decompressed.iter().zip(data.iter()).enumerate() {
            assert_eq!(a, b, "Mismatch at byte {}: got {} expected {} (dirty ring test)", i, a, b);
        }
    }

    // ── Background encode/decode tests ────────────────────

    #[test]
    fn encode_background_roundtrip() {
        let width: u32 = 8;
        let height: u32 = 8;
        let bpp: u32 = 4;
        let pixels: Vec<u8> = (0..(width * height * bpp) as u8).collect();

        let blob = encode_background(&pixels, width, height, bpp);

        // Verify structure: 1024 palette + 4 uncomp + 4 comp + compressed data
        assert!(blob.len() >= 1024 + 8);

        // Verify palette is all zeros
        assert!(blob[..1024].iter().all(|&b| b == 0));

        // Read back uncomp_size and comp_size
        let uncomp_size = i32::from_le_bytes(blob[1024..1028].try_into().unwrap()) as usize;
        let comp_size = i32::from_le_bytes(blob[1028..1032].try_into().unwrap()) as usize;
        assert_eq!(blob.len(), 1024 + 4 + 4 + comp_size);

        // Decompress and verify
        let decompressed = lzss_expand(&blob[1032..], uncomp_size).unwrap();
        let stride = i32::from_le_bytes(decompressed[0..4].try_into().unwrap()) as u32;
        let h = i32::from_le_bytes(decompressed[4..8].try_into().unwrap()) as u32;
        assert_eq!(stride, width * bpp);
        assert_eq!(h, height);
        assert_eq!(&decompressed[8..], &pixels);
    }

    // ── delete_background_image tests ──────────────────────

    #[test]
    fn delete_background_image_removes_file() {
        let dir = temp_project_dir("del_bg");
        let path = dir.to_string_lossy().to_string();
        let bg_dir = dir.join("Backgrounds");
        fs::create_dir_all(&bg_dir).unwrap();
        fs::write(bg_dir.join("Room1.png"), b"img").unwrap();

        let result = delete_background_image(path, "Room1.png".into()).unwrap();
        assert!(result);
        assert!(!bg_dir.join("Room1.png").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_background_image_missing_is_ok() {
        let dir = temp_project_dir("del_bg_miss");
        let path = dir.to_string_lossy().to_string();
        fs::create_dir_all(dir.join("Backgrounds")).unwrap();

        let result = delete_background_image(path, "Nonexistent.png".into()).unwrap();
        assert!(!result);
        let _ = fs::remove_dir_all(&dir);
    }

    // ── Synthetic .crm file for integration tests ─────────

    /// Build a minimal but valid .crm file matching AGS format version 33 (kRoomVersion_3508).
    /// Returns the bytes and the embedded background dimensions.
    fn build_synthetic_crm(bg_width: u32, bg_height: u32, bpp: u32) -> Vec<u8> {
        let mut crm = Vec::new();

        // ── File header: 2-byte version ──
        let version: u16 = 33; // kRoomVersion_3508
        crm.extend_from_slice(&version.to_le_bytes());

        // ── Main block (ID=1) ──
        let block_id: u8 = 1;
        crm.push(block_id);
        // Placeholder for 8-byte block length (version >= 32 uses i64)
        let len_offset = crm.len();
        crm.extend_from_slice(&0i64.to_le_bytes());

        let main_start = crm.len();

        // 1. BackgroundBPP (int32)
        crm.extend_from_slice(&(bpp as i32).to_le_bytes());

        // 2. WalkBehindCount (int16) = 1
        crm.extend_from_slice(&1i16.to_le_bytes());
        // Walk-behind baselines: 1 × int16
        crm.extend_from_slice(&0i16.to_le_bytes());

        // 3. HotspotCount (int32) = 1 (hotspot 0 is "no hotspot")
        let hotspot_count: i32 = 1;
        crm.extend_from_slice(&hotspot_count.to_le_bytes());

        // 4. Hotspot walk-to points: 1 × (int16 x, int16 y)
        crm.extend_from_slice(&0i16.to_le_bytes());
        crm.extend_from_slice(&0i16.to_le_bytes());

        // 5. Hotspot names (version >= 31: length-prefixed strings)
        for _ in 0..hotspot_count {
            crm.extend_from_slice(&0i32.to_le_bytes()); // empty string: len=0
        }

        // 6. Hotspot script names (version >= 31: length-prefixed)
        for _ in 0..hotspot_count {
            crm.extend_from_slice(&0i32.to_le_bytes()); // empty string: len=0
        }

        // 7. Legacy poly-point areas: int32 = 0
        crm.extend_from_slice(&0i32.to_le_bytes());

        // 8. Room edges: 4 × int16 (top, bottom, left, right)
        for _ in 0..4 {
            crm.extend_from_slice(&0i16.to_le_bytes());
        }

        // 9. Object count (int16) = 0
        let obj_count: i16 = 0;
        crm.extend_from_slice(&obj_count.to_le_bytes());
        // No objects to write

        // 10. Local variables: int32 count = 0
        crm.extend_from_slice(&0i32.to_le_bytes());

        // 11. Region count (int32) = 16 (MAX_ROOM_REGIONS)
        let region_count: i32 = 16;
        crm.extend_from_slice(&region_count.to_le_bytes());

        // 12. Event handlers: 1 room + 1 hotspot + 0 objects + 16 regions = 18 groups
        let groups = 1 + hotspot_count as usize + obj_count as usize + region_count as usize;
        for _ in 0..groups {
            // int32 event_count + null-terminated strings
            let evt_count: i32 = 7; // typical room event count
            crm.extend_from_slice(&evt_count.to_le_bytes());
            for _ in 0..evt_count {
                crm.push(0); // empty null-terminated string
            }
        }

        // 13. Object baselines: int32 × 0
        // (none, obj_count = 0)

        // 14. Room dimensions: int16 width, int16 height
        crm.extend_from_slice(&(bg_width as i16).to_le_bytes());
        crm.extend_from_slice(&(bg_height as i16).to_le_bytes());

        // 15. Object flags: int16 × 0
        // (none)

        // 16. MaskResolution (int16) = 1
        crm.extend_from_slice(&1i16.to_le_bytes());

        // 17. Walk area data: int32 count = 16 (MAX_WALK_AREAS)
        let walk_area_count: i32 = 16;
        crm.extend_from_slice(&walk_area_count.to_le_bytes());
        // 5 arrays × 16 × int16 (all zeros)
        for _ in 0..(5 * 16) {
            crm.extend_from_slice(&0i16.to_le_bytes());
        }

        // 18. Password: 11 zero bytes
        crm.extend_from_slice(&[0u8; 11]);

        // 19. Room options: 6 bytes + 4 reserved
        crm.extend_from_slice(&[0u8; 10]);

        // 20. MessageCount (int16) = 0
        crm.extend_from_slice(&0i16.to_le_bytes());

        // 21. GameID (int32) — version >= 25
        crm.extend_from_slice(&12345i32.to_le_bytes());

        // 22. Message info: 0 messages, nothing to write
        // 23. Messages: 0 messages, nothing to write

        // 24. Legacy animation count (int16) = 0
        crm.extend_from_slice(&0i16.to_le_bytes());

        // 25. Walk area PlayerView duplicate: 16 × int16
        for _ in 0..16 {
            crm.extend_from_slice(&0i16.to_le_bytes());
        }

        // 26. Region light levels: 16 × int16
        for _ in 0..16 {
            crm.extend_from_slice(&0i16.to_le_bytes());
        }

        // 27. Region tints: 16 × int32
        for _ in 0..16 {
            crm.extend_from_slice(&0i32.to_le_bytes());
        }

        // 28. Background image (LZSS compressed)
        let pixel_size = (bg_width * bg_height * bpp) as usize;
        let pixels = vec![0u8; pixel_size]; // black background
        let bg_blob = encode_background(&pixels, bg_width, bg_height, bpp);
        crm.extend_from_slice(&bg_blob);

        // 29. Area masks (RLE): 4 masks, each with int16 w, int16 h, minimal RLE data, 768-byte palette
        let mask_w = bg_width as i16;
        let mask_h = bg_height as i16;
        for _ in 0..4 {
            crm.extend_from_slice(&mask_w.to_le_bytes());
            crm.extend_from_slice(&mask_h.to_le_bytes());
            // Minimal RLE data: runs of zeros covering all pixels.
            // RLE control byte cx: if cx < 0, repeat next byte (1 - cx) times.
            // So for a run of length L: cx = 1 - L, range L=2..128 → cx=-1..-127.
            let total_pixels = (bg_width * bg_height) as usize;
            let mut remaining = total_pixels;
            while remaining > 0 {
                let run = remaining.min(128);
                let cx = (1i16 - run as i16) as i8;
                crm.push(cx as u8);
                crm.push(0); // value
                remaining -= run;
            }
            // 768-byte palette
            crm.extend_from_slice(&[0u8; 768]);
        }

        // ── EOF marker ──
        crm.push(0xFF);

        // Patch the Main block length
        // The masks are PART of the main block. Length = everything before EOF marker.
        let main_len = crm.len() - 1 - main_start;
        let len_bytes = (main_len as i64).to_le_bytes();
        crm[len_offset..len_offset + 8].copy_from_slice(&len_bytes);

        crm
    }

    #[test]
    fn find_background_in_synthetic_crm() {
        let crm = build_synthetic_crm(320, 200, 4);
        let version = u16::from_le_bytes([crm[0], crm[1]]);
        assert_eq!(version, 33);

        let (main_start, main_len) = find_main_block(&crm).unwrap();

        let bg = find_background_in_main(&crm, main_start, main_len, version).unwrap();
        assert_eq!(bg.bpp, 4);
        assert_eq!(bg.width, 320);
        assert_eq!(bg.height, 200);
        assert!(bg.offset > main_start);
        assert!(bg.offset + bg.total_size <= main_start + main_len);
    }

    #[test]
    fn embed_image_in_synthetic_crm() {
        let dir = temp_project_dir("embed_crm");
        let path = dir.to_string_lossy().to_string();

        // Write a synthetic 320x200 32-bit .crm
        let crm = build_synthetic_crm(320, 200, 4);
        fs::write(dir.join("room5.crm"), &crm).unwrap();

        // Create a 320x200 PNG image to embed
        let mut img = image::RgbaImage::new(320, 200);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = image::Rgba([(x & 0xFF) as u8, (y & 0xFF) as u8, 128, 255]);
        }
        let mut png_bytes = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
        image::ImageEncoder::write_image(
            encoder,
            img.as_raw(),
            320, 200,
            image::ExtendedColorType::Rgba8,
        ).unwrap();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

        // Embed
        let result = embed_image_in_crm(path.clone(), 5, b64).unwrap();
        assert!(result.contains("320x200"));
        assert!(result.contains("room5.crm"));

        // Verify the modified .crm can still be parsed
        let modified = fs::read(dir.join("room5.crm")).unwrap();
        let version = u16::from_le_bytes([modified[0], modified[1]]);
        let (main_start, main_len) = find_main_block(&modified).unwrap();
        let bg = find_background_in_main(&modified, main_start, main_len, version).unwrap();
        assert_eq!(bg.width, 320);
        assert_eq!(bg.height, 200);
        assert_eq!(bg.bpp, 4);

        // Verify the embedded pixel data is correct (decompress and check)
        let palette_end = bg.offset + 1024;
        let uncomp_size = read_i32(&modified, palette_end) as usize;
        let comp_size = read_i32(&modified, palette_end + 4) as usize;
        let comp_data = &modified[palette_end + 8..palette_end + 8 + comp_size];
        let decompressed = lzss_expand(comp_data, uncomp_size).unwrap();
        let stride = i32::from_le_bytes(decompressed[0..4].try_into().unwrap()) as u32;
        let height = i32::from_le_bytes(decompressed[4..8].try_into().unwrap()) as u32;
        assert_eq!(stride, 320 * 4);
        assert_eq!(height, 200);

        // Check first pixel: img pixel (0,0) = RGBA(0,0,128,255) → BGRA(128,0,0,255)
        assert_eq!(decompressed[8], 128); // B
        assert_eq!(decompressed[9], 0);   // G
        assert_eq!(decompressed[10], 0);  // R
        assert_eq!(decompressed[11], 255); // A

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn embed_dimension_mismatch_returns_error() {
        let dir = temp_project_dir("embed_mismatch");
        let path = dir.to_string_lossy().to_string();

        // 320x200 .crm
        let crm = build_synthetic_crm(320, 200, 4);
        fs::write(dir.join("room1.crm"), &crm).unwrap();

        // 640x480 PNG — wrong size
        let img = image::RgbaImage::new(640, 480);
        let mut png_bytes = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
        image::ImageEncoder::write_image(
            encoder,
            img.as_raw(),
            640, 480,
            image::ExtendedColorType::Rgba8,
        ).unwrap();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

        let result = embed_image_in_crm(path, 1, b64);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("dimensions"), "Error should mention dimensions: {}", err);
        assert!(err.contains("640x480"));
        assert!(err.contains("320x200"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn embed_empty_crm_returns_error() {
        let dir = temp_project_dir("embed_empty");
        let path = dir.to_string_lossy().to_string();

        // Write a 0-byte .crm
        fs::write(dir.join("room1.crm"), b"").unwrap();

        let img = image::RgbaImage::new(320, 200);
        let mut png_bytes = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
        image::ImageEncoder::write_image(
            encoder,
            img.as_raw(),
            320, 200,
            image::ExtendedColorType::Rgba8,
        ).unwrap();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

        let result = embed_image_in_crm(path, 1, b64);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("too small"), "Error should mention size: {}", err);

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
            copy_base_room_file,
            check_file_exists,
            check_background_matches,
            update_crm_room_events,
            list_crm_room_ids,
            copy_room_crm,
            save_base_room,
            copy_all_room_files,
            embed_image_in_crm,
            delete_background_image,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
