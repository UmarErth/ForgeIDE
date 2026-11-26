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
async fn ask_gemini(prompt: String, context: String, model: String) -> Result<String, String> {
    let key = key_entry()?.get_password().map_err(|_| "Add your Gemini API key in Settings to use AI.".to_string())?;
    let allowed_models = ["gemini-3.8-flash", "gemini-3.6-flash", "gemini-3.5-flash-lite", "gemini-4-hydrogen"];
    if !allowed_models.contains(&model.as_str()) {
        return Err("Choose a supported Gemini model from the model menu.".into());
    }
    let content = format!("Conversation and editor context:\n{context}\n\nUser request:\n{prompt}");
    let body = serde_json::json!({
        "model": model,
        "system_instruction": "You are Forge, a careful coding assistant inside a developer's code editor. Be concise, explain assumptions, and return code in Markdown fences when useful. When a task requires a local shell command, request exactly one command using this standalone format and no surrounding Markdown: <forge-command>{\"command\":\"...\",\"reason\":\"...\"}</forge-command>. Include a brief reason. Never claim you ran a command; Forge shows it to the user and runs it only after explicit approval. Do not request commands for examples or when an explanation suffices.",
        "input": content,
        "generation_config": { "temperature": 0.35, "max_output_tokens": 4096 }
    });
    let client = reqwest::Client::new();
    let response = client
        .post("https://generativelanguage.googleapis.com/v1beta/interactions")
        .header("x-goog-api-key", &key)
        .header("Api-Revision", "2026-05-20")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Could not reach Gemini: {e}"))?;
    let status = response.status();
    let mut data: serde_json::Value = response.json().await.map_err(|e| format!("Could not read Gemini response: {e}"))?;
    if !status.is_success() {
        let message = data.pointer("/error/message").and_then(|v| v.as_str()).unwrap_or("Gemini request failed");
        let code = data.pointer("/error/status").and_then(|v| v.as_str()).unwrap_or("");
        return Err(if code.is_empty() { format!("Gemini ({status}): {message}") } else { format!("Gemini ({status}, {code}): {message}") });
    }

    // Some API revisions wrap the Interaction resource, while others return it directly.
    if let Some(interaction) = data.get("interaction") {
        data = interaction.clone();
    }

    // Background interactions return an ID before their model output is ready. Fetch the
    // completed resource instead of reporting a misleading “no text” error.
    if data.get("status").and_then(|v| v.as_str()) == Some("in_progress") {
        let interaction_id = data.get("id").and_then(|v| v.as_str()).map(str::to_string);
        if let Some(id) = interaction_id {
            for _ in 0..60 {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                let poll = client
                    .get(format!("https://generativelanguage.googleapis.com/v1beta/interactions/{id}"))
                    .header("x-goog-api-key", &key)
                    .header("Api-Revision", "2026-05-20")
                    .send().await
                    .map_err(|e| format!("Could not check Gemini response: {e}"))?;
                let poll_status = poll.status();
                let polled: serde_json::Value = poll.json().await.map_err(|e| format!("Could not read Gemini response: {e}"))?;
                if !poll_status.is_success() {
                    let message = polled.pointer("/error/message").and_then(|v| v.as_str()).unwrap_or("Could not retrieve Gemini response");
                    return Err(format!("Gemini ({poll_status}): {message}"));
                }
                data = polled.get("interaction").cloned().unwrap_or(polled);
                match data.get("status").and_then(|v| v.as_str()) {
                    Some("completed") => break,
                    Some("failed" | "cancelled" | "requires_action") => break,
                    _ => {}
                }
            }
        }
    }

    if let Some(text) = data.get("output_text").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()) {
        return Ok(text.to_string());
    }

    fn collect_model_text(value: &serde_json::Value, in_model_output: bool, parts: &mut Vec<String>) {
        match value {
            serde_json::Value::Array(items) => {
                for item in items { collect_model_text(item, in_model_output, parts); }
            }
            serde_json::Value::Object(map) => {
                let kind = map.get("type").and_then(|v| v.as_str()).unwrap_or("");
                let is_model_output = in_model_output || kind == "model_output";
                if kind == "text" && is_model_output {
                    if let Some(text) = map.get("text").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()) {
                        parts.push(text.to_string());
                    }
                    return;
                }
                for (key, child) in map {
                    if key == "text" && is_model_output {
                        if let Some(text) = child.as_str().filter(|s| !s.trim().is_empty()) {
                            parts.push(text.to_string());
                        }
                    } else if key != "text" {
                        collect_model_text(child, is_model_output, parts);
                    }
                }
            }
            _ => {}
        }
    }

    let mut text_parts = Vec::new();
    collect_model_text(&data, false, &mut text_parts);
    if !text_parts.is_empty() {
        return Ok(text_parts.join("\n"));
    }

    let response_status = data.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
    let failure = data.get("error").and_then(|e| e.get("message")).and_then(|v| v.as_str());
    if let Some(message) = failure {
        return Err(format!("Gemini response status {response_status}: {message}"));
    }
    if response_status == "in_progress" {
        return Err("Gemini is still generating a response after 60 seconds. Try again.".into());
    }
    let step_types = data.get("steps").and_then(|v| v.as_array())
        .map(|steps| steps.iter().filter_map(|step| step.get("type").and_then(|v| v.as_str())).collect::<Vec<_>>().join(", "))
        .filter(|s| !s.is_empty()).unwrap_or_else(|| "none".into());
    Err(format!("Gemini returned no text (response status: {response_status}; response steps: {step_types}). Try again."))
}

struct RunningCommand(std::sync::Mutex<Option<std::process::Child>>);

fn read_command_stream(mut reader: impl std::io::Read) -> String {
    let mut output = Vec::new();
    let mut chunk = [0u8; 8192];
    let mut truncated = false;
    loop {
        match reader.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                let remaining = 1_000_000usize.saturating_sub(output.len());
                let keep = remaining.min(n);
                output.extend_from_slice(&chunk[..keep]);
                if keep < n { truncated = true; }
            }
        }
    }
    let mut text = String::from_utf8_lossy(&output).to_string();
    if truncated { text.push_str("\n… output truncated after 1 MB …\n"); }
    text
}

#[tauri::command]
fn run_command(command: String, cwd: Option<String>, state: tauri::State<'_, RunningCommand>) -> Result<String, String> {
    let command = command.trim();
    if command.is_empty() { return Err("Type a command to run.".into()); }
    if command.len() > 16_384 { return Err("Command is too long (maximum 16,384 characters).".into()); }
    {
        let active = state.0.lock().map_err(|_| "Command runner is unavailable.".to_string())?;
        if active.is_some() { return Err("A command is already running.".into()); }
    }
    let working_dir = match cwd {
        Some(path) => {
            let path = std::path::PathBuf::from(path);
            if !path.is_dir() { return Err("The selected working folder no longer exists.".into()); }
            path
        }
        None => std::env::current_dir().map_err(|e| format!("Could not determine the working folder: {e}"))?,
    };
    #[cfg(target_os = "windows")]
    let mut child = std::process::Command::new("cmd.exe").args(["/C", command]).current_dir(&working_dir).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().map_err(|e| format!("Could not start command: {e}"))?;
    #[cfg(not(target_os = "windows"))]
    let mut child = std::process::Command::new("/bin/sh").args(["-lc", command]).current_dir(&working_dir).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().map_err(|e| format!("Could not start command: {e}"))?;
    let stdout = child.stdout.take().ok_or_else(|| "Could not read command output.".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "Could not read command errors.".to_string())?;
    let out_reader = std::thread::spawn(move || read_command_stream(stdout));
    let err_reader = std::thread::spawn(move || read_command_stream(stderr));
    {
        let mut active = state.0.lock().map_err(|_| "Command runner is unavailable.".to_string())?;
        if active.is_some() {
            let _ = child.kill();
            return Err("A command is already running.".into());
        }
        *active = Some(child);
    }
    let status = loop {
        let result = {
            let mut active = state.0.lock().map_err(|_| "Command runner is unavailable.".to_string())?;
            match active.as_mut() {
                Some(child) => child.try_wait().map_err(|e| format!("Could not check command status: {e}"))?,
                None => return Err("Command was stopped.".into()),
            }
        };
        if let Some(status) = result { break status; }
        std::thread::sleep(std::time::Duration::from_millis(80));
    };
    let _ = state.0.lock().map_err(|_| "Command runner is unavailable.".to_string())?.take();
    let stdout = out_reader.join().unwrap_or_else(|_| "Could not collect command output.".into());
    let stderr = err_reader.join().unwrap_or_else(|_| "Could not collect command errors.".into());
    let mut result = String::new();
    if !stdout.is_empty() { result.push_str(&stdout); }
    if !stderr.is_empty() {
        if !result.is_empty() && !result.ends_with('\n') { result.push('\n'); }
        result.push_str(&stderr);
    }
    if !status.success() {
        if !result.is_empty() && !result.ends_with('\n') { result.push('\n'); }
        result.push_str(&format!("[process exited with {}]", status.code().map(|c| c.to_string()).unwrap_or_else(|| "no exit code".into())));
    } else if result.is_empty() {
        result.push_str("[command completed successfully with no output]");
    }
    Ok(result)
}

#[tauri::command]
fn stop_command(state: tauri::State<'_, RunningCommand>) -> Result<bool, String> {
    let mut active = state.0.lock().map_err(|_| "Command runner is unavailable.".to_string())?;
    match active.as_mut() {
        Some(child) => { child.kill().map_err(|e| format!("Could not stop command: {e}"))?; Ok(true) }
        None => Ok(false),
    }
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
        .manage(RunningCommand(std::sync::Mutex::new(None)))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![save_api_key, api_key_status, get_profile_name, save_profile_name, search_project, git_status, ask_gemini, read_text_file, write_text_file, list_directory, run_command, stop_command])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Regular);
            if let Some(w) = app.get_webview_window("main") { w.set_title("Forge — AI Code Studio")?; }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Forge IDE");
}
