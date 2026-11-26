use keyring::Entry;
use serde::{Deserialize, Serialize};
use tauri::Manager;

const SERVICE: &str = "forge-ide";
const ACCOUNT: &str = "gemini-api-key";
const PROFILE_ACCOUNT: &str = "profile-name";

fn key_entry() -> Result<Entry, String> {
    Entry::new(SERVICE, ACCOUNT).map_err(|e| format!("Could not open secure key storage: {e}"))
}

#[tauri::command]
fn save_api_key(api_key: String) -> Result<(), String> {
    let key = api_key.trim();
    if key.is_empty() { return Err("Enter a Gemini API key first.".into()); }
    key_entry()?.set_password(key).map_err(|e| format!("Could not save API key securely: {e}"))
}

#[tauri::command]
fn api_key_status() -> bool { key_entry().and_then(|e| e.get_password().map_err(|e| e.to_string())).is_ok() }

#[tauri::command]
fn save_profile_name(name: String) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() || name.len() > 48 { return Err("Enter a name between 1 and 48 characters.".into()); }
    Entry::new(SERVICE, PROFILE_ACCOUNT).map_err(|e| e.to_string())?.set_password(name).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_profile_name() -> Option<String> {
    Entry::new(SERVICE, PROFILE_ACCOUNT).ok()?.get_password().ok()
}

#[derive(Serialize)]
struct SearchMatch { path: String, name: String, relative_path: String, line: usize, preview: String }

#[tauri::command]
fn search_project(root: String, query: String) -> Result<Vec<SearchMatch>, String> {
    let needle = query.to_lowercase();
    let root_path = std::path::PathBuf::from(&root);
    let mut stack = vec![root_path.clone()];
    let mut matches = Vec::new();
    let mut visited = 0usize;
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) { Ok(v) => v, Err(_) => continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || matches!(name.as_str(), "target" | "node_modules" | "vendor" | "dist" | "build") { continue; }
            if path.is_dir() { stack.push(path); continue; }
            visited += 1;
            if visited > 5000 || matches.len() >= 300 { return Ok(matches); }
            let meta = match entry.metadata() { Ok(v) => v, Err(_) => continue };
            if meta.len() > 1_000_000 { continue; }
            let content = match std::fs::read_to_string(&path) { Ok(v) => v, Err(_) => continue };
            let rel = path.strip_prefix(&root_path).unwrap_or(&path).to_string_lossy().to_string();
            for (index, line) in content.lines().enumerate() {
                if line.to_lowercase().contains(&needle) {
                    matches.push(SearchMatch { path: path.to_string_lossy().to_string(), name: name.clone(), relative_path: rel.clone(), line: index + 1, preview: line.trim().chars().take(180).collect() });
                    if matches.len() >= 300 { return Ok(matches); }
                }
            }
        }
    }
    Ok(matches)
}

#[tauri::command]
fn git_status(root: String) -> Result<String, String> {
    let output = std::process::Command::new("git").args(["-C", &root, "status", "--short", "--branch"]).output().map_err(|e| format!("Could not run Git: {e}"))?;
    if output.status.success() { Ok(String::from_utf8_lossy(&output.stdout).to_string()) }
    else { let error = String::from_utf8_lossy(&output.stderr).trim().to_string(); Err(if error.is_empty() { "This folder is not a Git repository.".into() } else { error }) }
}

#[tauri::command]
async fn ask_gemini(prompt: String, context: String) -> Result<String, String> {
    let key = key_entry()?.get_password().map_err(|_| "Add your Gemini API key in Settings to use AI.".to_string())?;
    let content = format!("Editor context:\n{context}\n\nUser request:\n{prompt}");
    let body = serde_json::json!({
        "model": "gemini-3.8-flash",
        "system_instruction": "You are Forge, a careful coding assistant inside a developer's code editor. Be concise, explain assumptions, and return code in Markdown fences when useful.",
        "input": content,
        "generation_config": { "temperature": 0.35, "max_output_tokens": 4096 }
    });
    let response = reqwest::Client::new()
        .post("https://generativelanguage.googleapis.com/v1beta/interactions")
        .header("x-goog-api-key", key)
        .header("Api-Revision", "2026-05-20")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Could not reach Gemini: {e}"))?;
    let status = response.status();
    let data: serde_json::Value = response.json().await.map_err(|e| format!("Could not read Gemini response: {e}"))?;
    if !status.is_success() {
        let message = data.pointer("/error/message").and_then(|v| v.as_str()).unwrap_or("Gemini request failed");
        let code = data.pointer("/error/status").and_then(|v| v.as_str()).unwrap_or("");
        return Err(if code.is_empty() { format!("Gemini ({status}): {message}") } else { format!("Gemini ({status}, {code}): {message}") });
    }
    if let Some(text) = data.get("output_text").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()) {
        return Ok(text.to_string());
    }
    let text = data.pointer("/output").and_then(|v| v.as_array())
        .map(|items| items.iter().filter_map(|item| {
            if item.get("type").and_then(|v| v.as_str()) == Some("text") {
                item.get("text").and_then(|v| v.as_str())
            } else {
                None
            }
        }).collect::<Vec<_>>().join("\n"))
        .filter(|s| !s.trim().is_empty());
    text.ok_or_else(|| {
        let detail = data.pointer("/error/message").and_then(|v| v.as_str());
        detail.map(|m| format!("Gemini returned no text: {m}")).unwrap_or_else(|| "Gemini returned no text. Try again.".into())
    })
}

#[derive(Serialize, Deserialize)]
struct EditorFile { name: String, path: String, content: String }

#[tauri::command]
fn read_text_file(path: String) -> Result<EditorFile, String> {
    let content = std::fs::read_to_string(&path).map_err(|e| format!("Could not open file: {e}"))?;
    let name = std::path::Path::new(&path).file_name().and_then(|v| v.to_str()).unwrap_or("untitled").to_string();
    Ok(EditorFile { name, path, content })
}

#[tauri::command]
fn write_text_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(path, content).map_err(|e| format!("Could not save file: {e}"))
}

#[tauri::command]
fn list_directory(path: String) -> Result<Vec<serde_json::Value>, String> {
    let mut rows = Vec::new();
    for entry in std::fs::read_dir(&path).map_err(|e| format!("Could not open folder: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let meta = entry.metadata().map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "target" || name == "node_modules" { continue; }
        rows.push(serde_json::json!({"name":name,"path":entry.path().to_string_lossy(),"directory":meta.is_dir()}));
    }
    rows.sort_by(|a,b| b["directory"].as_bool().cmp(&a["directory"].as_bool()).then(a["name"].as_str().cmp(&b["name"].as_str())));
    Ok(rows)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![save_api_key, api_key_status, get_profile_name, save_profile_name, search_project, git_status, ask_gemini, read_text_file, write_text_file, list_directory])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Regular);
            if let Some(w) = app.get_webview_window("main") { w.set_title("Forge — AI Code Studio")?; }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Forge IDE");
}
