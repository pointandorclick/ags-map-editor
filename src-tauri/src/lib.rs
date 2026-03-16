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
fn check_file_exists(file_path: String) -> Result<bool, String> {
    Ok(Path::new(&file_path).exists())
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
            check_file_exists,
            check_background_matches,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
