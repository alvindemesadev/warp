use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::Instant;
use tauri::{Emitter, Manager, Window};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// ── State ─────────────────────────────────────────────────────────────────────
struct ActiveProcess(Mutex<Option<std::process::Child>>);

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WarpProgress {
    pub percentage: u32,
    pub current_file: String,
    pub speed: String,
    pub files_done: u32,
    pub files_total: u32,
    pub indeterminate: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WarpSummary {
    pub total_files: u32,
    pub transferred: u32,
    pub skipped: u32,
    pub failed: u32,
    pub duration_ms: u64,
    pub bytes_transferred: u64,
    pub cancelled: bool,
    pub error_code: i32,
    pub error_message: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PathMeta {
    pub files: u64,
    pub bytes: u64,
    pub is_file: bool,
    pub drive: String, // e.g. "C:" for cross-drive detection
}

// ── Command factory ───────────────────────────────────────────────────────────

fn robocopy_cmd() -> Command {
    let mut c = Command::new("robocopy");
    #[cfg(windows)]
    c.creation_flags(CREATE_NO_WINDOW);
    c.stdout(Stdio::piped()).stderr(Stdio::null());
    c
}

// ── Path info ─────────────────────────────────────────────────────────────────

#[tauri::command]
async fn get_path_info(path: String) -> Result<PathMeta, String> {
    let meta = std::fs::metadata(&path).map_err(|e| e.to_string())?;

    // Extract drive letter (Windows: "C:", "D:", etc.)
    let drive = std::path::Path::new(&path)
        .components()
        .next()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .unwrap_or_default();

    if meta.is_file() {
        return Ok(PathMeta {
            files: 1,
            bytes: meta.len(),
            is_file: true,
            drive,
        });
    }

    let mut count = 0u64;
    let mut bytes = 0u64;
    walk_dir(&path, &mut count, &mut bytes);

    Ok(PathMeta {
        files: count,
        bytes,
        is_file: false,
        drive,
    })
}

fn walk_dir(dir: &str, count: &mut u64, bytes: &mut u64) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if let Ok(m) = e.metadata() {
                if m.is_file() {
                    *count += 1;
                    *bytes += m.len();
                } else if m.is_dir() {
                    walk_dir(&e.path().to_string_lossy(), count, bytes);
                }
            }
        }
    }
}

// ── Cancel ────────────────────────────────────────────────────────────────────

#[tauri::command]
async fn cancel_warp(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<ActiveProcess>();
    let mut guard = state.0.lock().unwrap();
    if let Some(child) = guard.as_mut() {
        let _ = child.kill();
        let _ = child.wait();
    }
    *guard = None;
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn overall_pct(done: u64, total: u64) -> u32 {
    if total == 0 { return 0; }
    ((done as f64 / total as f64) * 100.0).clamp(0.0, 99.0) as u32
}

fn fmt_speed(bps: u64) -> String {
    if bps >= 1_073_741_824 { format!("{:.1} GB/s", bps as f64 / 1_073_741_824.0) }
    else if bps >= 1_048_576 { format!("{:.0} MB/s", bps as f64 / 1_048_576.0) }
    else if bps >= 1_024     { format!("{:.0} KB/s", bps as f64 / 1_024.0) }
    else                     { format!("{} B/s", bps) }
}

fn basename(path: &str) -> String {
    path.replace('\\', "/")
        .split('/')
        .filter(|s| !s.is_empty())
        .last()
        .unwrap_or(path)
        .to_string()
}

/// Translate robocopy exit codes to human-readable messages.
/// Codes 0-7 are success/info. 8+ are real failures.
fn robocopy_exit_message(code: i32) -> Option<String> {
    match code {
        0..=7 => None, // success
        8  => Some("Some files or directories could not be copied (copy errors occurred and the retry limit was exceeded). Check permissions or disk space.".to_string()),
        16 => Some("Robocopy did not copy any files. Check the source and destination paths.".to_string()),
        _ => {
            // Check for disk-full indicators in common codes
            if code & 8 != 0 {
                Some(format!("Transfer failed (exit code {}). Possible causes: disk full, access denied, or path too long.", code))
            } else {
                Some(format!("Transfer failed with exit code {}.", code))
            }
        }
    }
}

// ── Parser ────────────────────────────────────────────────────────────────────

enum RoboLine {
    FileHeader { is_same: bool, is_error: bool, size: u64, name: String },
    #[allow(dead_code)]
    Pct(f64),
    Speed(u64),
    Skip,
}

fn parse_line(raw: &str) -> RoboLine {
    let t = raw.trim();
    if t.is_empty() { return RoboLine::Skip; }

    if t.to_lowercase().contains("bytes/sec") {
        for tok in t.split_whitespace() {
            if let Ok(bps) = tok.replace(',', "").parse::<u64>() {
                if bps > 1000 { return RoboLine::Speed(bps); }
            }
        }
        return RoboLine::Skip;
    }

    {
        let toks: Vec<&str> = t.split_whitespace().collect();
        if toks.len() == 1 && toks[0].ends_with('%') {
            if let Ok(p) = toks[0].trim_end_matches('%').parse::<f64>() {
                if (0.0..=100.0).contains(&p) { return RoboLine::Pct(p); }
            }
        }
    }

    let known = [
        ("New File", false, false),
        ("Newer",    false, false),
        ("Older",    false, false),
        ("Same",     true,  false),
        ("ERROR",    false, true ),
    ];

    for (tag, is_same, is_error) in &known {
        if let Some(after) = t.find(tag).map(|p| &t[p + tag.len()..]) {
            let mut parts = after.split_whitespace();
            if let Some(size_str) = parts.next() {
                if let Ok(size) = size_str.parse::<u64>() {
                    let rest: Vec<&str> = parts.collect();
                    let name = if rest.is_empty() { String::new() } else { basename(&rest.join(" ")) };
                    return RoboLine::FileHeader { is_same: *is_same, is_error: *is_error, size, name };
                }
            }
        }
    }

    RoboLine::Skip
}

// ── Scan pass ─────────────────────────────────────────────────────────────────

fn scan(source: &str, destination: &str) -> (u64, u32) {
    let out = match robocopy_cmd()
        .args([source, destination, "/L", "/E", "/BYTES", "/NJH", "/NJS", "/NP"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return (0, 0),
    };

    let mut total_bytes = 0u64;
    let mut total_files = 0u32;
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if let RoboLine::FileHeader { size, is_error: false, .. } = parse_line(line) {
            total_bytes += size;
            total_files += 1;
        }
    }
    (total_bytes, total_files)
}

// ── Main transfer command ─────────────────────────────────────────────────────

#[tauri::command]
async fn warp_file_op(
    window: Window,
    app: tauri::AppHandle,
    source: String,
    destination: String,
    mode: String,
    conflict: String,
    folder_mode: String, // "into" | "merge"
) -> Result<WarpSummary, String> {

    // ── Destination path resolution ───────────────────────────────────────────
    //
    // folder_mode = "into":  source=C:\Photos\Screenshots, dest=C:\Backup
    //   → robocopy copies INTO C:\Backup\Screenshots\
    //   BUT only if dest does NOT already end with the source folder name.
    //   If user drops C:\Backup\Screenshots as dest (already the right folder),
    //   do NOT append again → avoid C:\Backup\Screenshots\Screenshots
    //
    // folder_mode = "merge": copy contents directly into dest, no subfolder.

    let source_name = std::path::Path::new(&source)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    let effective_dest = if source_name.is_empty() || folder_mode == "merge" {
        // Merge mode: copy contents straight into destination
        destination.clone()
    } else {
        // "Into" mode: append source folder name — but only if the destination
        // doesn't already end with that name (prevents double-nesting).
        let dest_clean = destination.trim_end_matches('\\');
        let dest_last = std::path::Path::new(dest_clean)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if dest_last.eq_ignore_ascii_case(&source_name) {
            // Destination already IS the target folder (e.g. user dropped Screenshots onto Screenshots)
            destination.clone()
        } else {
            format!("{}\\{}", dest_clean, source_name)
        }
    };

    // Scan for total size (determines whether progress bar is determinate)
    let (total_bytes, total_files_scan) = scan(&source, &effective_dest);
    let indeterminate = total_bytes == 0; // empty folder or all zero-byte files

    let mut args = vec![source.clone(), effective_dest.clone()];

    match mode.as_str() {
        "move" => args.push("/MOVE".to_string()),
        "sync" => args.push("/MIR".to_string()),
        _ => {}
    }

    if conflict == "skip" {
        args.push("/XO".to_string());
        args.push("/XN".to_string());
    }

    args.extend([
        "/E".to_string(),
        "/MT:32".to_string(),
        "/NP".to_string(),
        "/R:3".to_string(),
        "/W:5".to_string(),
        "/BYTES".to_string(),
        "/NJH".to_string(),
        "/NJS".to_string(),
        "/256".to_string(), // support paths longer than 260 chars (fix #3)
    ]);

    // Spawn
    let mut child = robocopy_cmd()
        .args(&args)
        .spawn()
        .map_err(|e| format!("Failed to start robocopy: {e}"))?;

    let stdout = child.stdout.take();

    {
        let state = app.state::<ActiveProcess>();
        *state.0.lock().unwrap() = Some(child);
    }

    let start = Instant::now();
    let mut summary = WarpSummary {
        total_files: 0, transferred: 0, skipped: 0, failed: 0,
        duration_ms: 0, bytes_transferred: 0, cancelled: false,
        error_code: 0, error_message: String::new(),
    };

    let mut bytes_done: u64 = 0;
    let mut last_emitted: u32 = 0;
    let mut last_speed_str = String::new();
    let mut files_done_count: u32 = 0;

    // Live speed tracking
    let mut speed_window_bytes: u64 = 0;
    let mut speed_window_start = Instant::now();

    // For indeterminate mode: emit a "pulse" every N files so UI shows activity
    let mut indeterminate_tick: u32 = 0;

    if let Some(stdout) = stdout {
        for line in BufReader::new(stdout).lines().flatten() {
            // Cancelled check
            {
                let state = app.state::<ActiveProcess>();
                if state.0.lock().unwrap().is_none() {
                    summary.cancelled = true;
                    break;
                }
            }

            match parse_line(&line) {
                RoboLine::FileHeader { is_same, is_error, size, name } => {
                    if is_error {
                        summary.failed += 1;
                        let _ = window.emit("warp-error", name.clone());
                    } else if is_same {
                        summary.skipped += 1;
                    } else {
                        summary.transferred += 1;
                    }
                    summary.total_files += 1;
                    files_done_count += 1;
                    bytes_done = bytes_done.saturating_add(size);
                    summary.bytes_transferred = bytes_done;

                    // Live speed
                    speed_window_bytes += size;
                    let window_ms = speed_window_start.elapsed().as_millis() as u64;
                    if window_ms >= 400 {
                        let bps = (speed_window_bytes as f64 / window_ms as f64 * 1000.0) as u64;
                        if bps > 0 { last_speed_str = fmt_speed(bps); }
                        speed_window_bytes = 0;
                        speed_window_start = Instant::now();
                    }

                    let pct = if indeterminate {
                        // Emit fake "pulse" increments for empty/tiny folders
                        indeterminate_tick = (indeterminate_tick + 1) % 100;
                        indeterminate_tick
                    } else {
                        overall_pct(bytes_done, total_bytes)
                    };

                    if pct != last_emitted || !name.is_empty() {
                        last_emitted = pct;
                        let _ = window.emit("warp-progress", WarpProgress {
                            percentage: pct,
                            current_file: name,
                            speed: last_speed_str.clone(),
                            files_done: files_done_count,
                            files_total: total_files_scan,
                            indeterminate,
                        });
                    }
                }

                RoboLine::Pct(_) => {}

                RoboLine::Speed(bps) => {
                    if last_speed_str.is_empty() {
                        last_speed_str = fmt_speed(bps);
                    }
                }

                RoboLine::Skip => {}
            }
        }
    }

    // Get exit code
    let code = {
        let state = app.state::<ActiveProcess>();
        let mut guard = state.0.lock().unwrap();
        let c = if let Some(ref mut child) = *guard {
            child.wait().ok().and_then(|s| s.code()).unwrap_or(0)
        } else {
            0
        };
        *guard = None;
        c
    };

    summary.duration_ms = start.elapsed().as_millis() as u64;
    summary.error_code = code;

    if summary.cancelled { return Ok(summary); }

    if code < 8 {
        // For move mode: robocopy /MOVE removes files but may leave empty
        // source directories behind. Clean them up.
        if mode == "move" && summary.failed == 0 {
            let _ = std::fs::remove_dir_all(&source);
        }
        Ok(summary)
    } else {
        // Surface a meaningful error message (#4 disk full / access denied)
        summary.error_message = robocopy_exit_message(code)
            .unwrap_or_else(|| format!("Transfer failed (exit code {})", code));
        Ok(summary) // Return as Ok with error info, not Err — so UI gets the summary
    }
}

// ── Entry ─────────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ActiveProcess(Mutex::new(None)))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            warp_file_op,
            get_path_info,
            cancel_warp
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
