#[tauri::command]
pub fn import_file(path: String) -> Result<String, String> {
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err(format!("File not found: {path}"));
    }
    let ext = p.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !["wav", "aiff", "aif", "mp3"].contains(&ext.as_str()) {
        return Err(format!("Unsupported format: .{ext}"));
    }
    p.canonicalize()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}
