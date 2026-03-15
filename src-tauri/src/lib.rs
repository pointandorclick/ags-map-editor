use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

#[derive(Serialize, Deserialize, Clone)]
struct RecentProject {
    path: String,
    name: String,
}

#[tauri::command]
fn pick_project_folder(app_handle: AppHandle) -> Result<String, String> {
    let folder = app_handle.dialog().file().blocking_pick_folder();

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
