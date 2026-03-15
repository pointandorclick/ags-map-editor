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
            check_file_exists,
            check_background_matches,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
