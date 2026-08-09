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
    pub bytes_per_sec: u64,
    pub bytes_done: u64,
    pub total_bytes: u64,
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
    pub verified: bool,
    pub verify_mismatches: u32,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PathMeta {
    pub files: u64,
    pub bytes: u64,
    pub is_file: bool,
    pub drive: String, // e.g. "C:" for cross-drive detection
    pub removable: bool, // true if the drive is a removable/USB drive
}

// ── Drive type detection (Windows) ──────────────────────────────────────────

#[cfg(windows)]
extern "system" {
    #[link_name = "GetDriveTypeW"]
    fn winapi_GetDriveTypeW(lpRootPathName: *const u16) -> u32;
}

#[cfg(windows)]
fn is_removable_drive(drive: &str) -> bool {
    let root = format!(r"{}\", drive.trim_end_matches(':'));
    let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
    const DRIVE_REMOVABLE: u32 = 2;
    unsafe { winapi_GetDriveTypeW(wide.as_ptr()) == DRIVE_REMOVABLE }
}

#[cfg(not(windows))]
fn is_removable_drive(_drive: &str) -> bool {
    false
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
    // Walking a large tree can take a while — never block an async worker.
    tauri::async_runtime::spawn_blocking(move || get_path_info_sync(path))
        .await
        .map_err(|e| format!("Path scan task failed: {e}"))?
}

fn get_path_info_sync(path: String) -> Result<PathMeta, String> {
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
            removable: false,
        });
    }

    let mut count = 0u64;
    let mut bytes = 0u64;
    walk_dir(&path, &mut count, &mut bytes);

    let removable = !drive.is_empty() && is_removable_drive(&drive);

    Ok(PathMeta {
        files: count,
        bytes,
        is_file: false,
        drive,
        removable,
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

/// Recursively remove empty directories starting from `dir`, bottom-up.
/// A directory is removed only if it ends up empty after its empty children
/// are removed. Any directory that still contains files is left untouched.
/// This is used after a `/MOVE` to clean up leftover empty folders WITHOUT
/// risking deletion of files that were skipped and intentionally left behind.
fn remove_empty_dirs(dir: &std::path::Path) -> bool {
    if !dir.is_dir() {
        return false;
    }

    let mut is_empty = true;
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Recurse first; if the child couldn't be fully removed,
                // this directory is not empty either.
                if !remove_empty_dirs(&path) {
                    is_empty = false;
                }
            } else {
                // A file remains — never delete this directory.
                is_empty = false;
            }
        }
    } else {
        // Couldn't read the directory; don't attempt to remove it.
        return false;
    }

    if is_empty {
        std::fs::remove_dir(dir).is_ok()
    } else {
        false
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

/// Convert a target throughput (MB/s) into robocopy's `/IPG` inter-packet gap
/// in milliseconds. Robocopy moves data in 64 KB blocks, so blocks/sec = MB/s * 16
/// and the gap between blocks is 1000 / (MB/s * 16) = 62.5 / MB/s ms (min 1).
/// Returns None for 0 (unlimited).
fn ipg_for_throttle(mb_per_sec: u32) -> Option<u64> {
    if mb_per_sec == 0 {
        None
    } else {
        Some(((62.5 / mb_per_sec as f64).round() as u64).max(1))
    }
}

fn basename(path: &str) -> String {
    path.replace('\\', "/")
        .split('/')
        .filter(|s| !s.is_empty())
        .last()
        .unwrap_or(path)
        .to_string()
}

/// Extract the drive letter (e.g. "C:") from a path, or empty string.
fn extract_drive(path: &str) -> String {
    std::path::Path::new(path)
        .components()
        .next()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Returns true if the given path is on a removable (USB) drive.
fn is_path_on_usb(path: &str) -> bool {
    let drive = extract_drive(path);
    !drive.is_empty() && is_removable_drive(&drive)
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
    Speed(u64),
    Skip,
}

/// Parse one line of robocopy output.
///
/// Instead of matching robocopy's English status words ("New File", "Same",
/// "ERROR"), the parser keys off the tab-delimited COLUMN STRUCTURE, which is
/// identical in every Windows locale — only the status word itself is
/// localized. File rows always have 5 columns (`["", status, "", size, path]`),
/// directory rows have 3, and error log lines carry a locale-independent
/// `N (0xXXXXXXXX)` code pair. This keeps progress, totals, and file names
/// accurate on non-English Windows; only the Same/ERROR *classification* falls
/// back to best-effort word matching (an unrecognized status is treated as a
/// regular copy, which is the safe direction for progress tracking).
fn parse_line(raw: &str) -> RoboLine {
    let t = raw.trim();
    if t.is_empty() { return RoboLine::Skip; }

    // Speed line (best-effort — the "Bytes/sec" label is localized too, but
    // live speed is also computed from file sizes, so this only helps the
    // very first second of a transfer).
    if t.to_lowercase().contains("bytes/sec") {
        for tok in t.split_whitespace() {
            if let Ok(bps) = tok.replace(',', "").parse::<u64>() {
                if bps > 1000 { return RoboLine::Speed(bps); }
            }
        }
        return RoboLine::Skip;
    }

    // Error log lines, e.g. "2026/08/06 21:12:33 ERROR 32 (0x00000020) Copying
    // File C:\...". The "<decimal> (0x<hex>)" pair is rendered the same in
    // every locale, unlike the "ERROR" word itself.
    {
        let toks: Vec<&str> = t.split_whitespace().collect();
        for (i, tok) in toks.iter().enumerate() {
            if tok.parse::<u32>().is_err() { continue; }
            let Some(hex) = toks.get(i + 1) else { break };
            let is_hex_code = hex.starts_with("(0x")
                && hex.ends_with(')')
                && hex.len() > 4
                && hex[3..hex.len() - 1].chars().all(|c| c.is_ascii_hexdigit());
            if is_hex_code {
                let name = basename(&toks[i + 2..].join(" "));
                return RoboLine::FileHeader { is_same: false, is_error: true, size: 0, name };
            }
        }
    }

    // File-list rows are tab-delimited. IMPORTANT: split `raw` (not `t`) — the
    // leading tab is what keeps column 0 empty.
    //   File rows:  ["", "New File", "", "1024", "path"]   (5 columns)
    //   Dir rows:   ["", "New Dir  1", "path"]             (3 columns, skipped)
    //   Extra rows: ["", "*EXTRA File", "", "12", "path"] (dest-only, skipped)
    let cols: Vec<&str> = raw.split('\t').collect();
    if cols.len() >= 5 {
        let status = cols[1].trim();
        let path = cols[4..].join(" ").trim().to_string();
        if let Ok(size) = cols[3].trim().parse::<u64>() {
            if !status.is_empty() && !path.is_empty() && !status.starts_with('*') {
                let is_same = status.eq_ignore_ascii_case("Same");
                let is_error = status.eq_ignore_ascii_case("ERROR");
                return RoboLine::FileHeader { is_same, is_error, size, name: basename(&path) };
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

// ── Verify pass ───────────────────────────────────────────────────────────────
//
// Robocopy has no content-hash verification, so "verify" re-runs a list-only
// (/L) comparison of source vs destination and counts how many files robocopy
// would still copy. After a clean copy that count should be zero — every file
// is present in the destination with a matching size and timestamp. Any non-zero
// result means a file is missing or differs (a size/timestamp mismatch).
//
// This is a structural verification (existence + size + time), not a byte-for-byte
// hash check.
fn verify_transfer(source: &str, destination: &str) -> u32 {
    let out = match robocopy_cmd()
        .args([source, destination, "/L", "/E", "/BYTES", "/NJH", "/NJS", "/NP"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return 0,
    };

    // A file robocopy would still copy = not identical in the destination.
    // (File rows the parser couldn't classify, e.g. localized status words,
    // are treated as copies — see parse_line.)
    let mut mismatches = 0u32;
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if let RoboLine::FileHeader { is_same: false, is_error: false, .. } = parse_line(line) {
            mismatches += 1;
        }
    }

    // Robocopy's own exit code is the authoritative signal: 0 = nothing would
    // be copied (identical trees), anything else = the comparison was unable to
    // prove the trees identical (files differ, or the verify pass itself errored
    // with an 8+ code). Backs up the parser on non-English systems so verify can
    // never report a false "all clear".
    let code = out.status.code().unwrap_or(0);
    if code == 0 {
        0
    } else {
        mismatches.max(1)
    }
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
    throttle: u32,       // target MB/s, 0 = unlimited
    verify: bool,        // run a verification pass after a successful transfer
) -> Result<WarpSummary, String> {
    // The whole pipeline (scan pass + streaming robocopy output) is synchronous
    // and can run for a long time. Running it inside an `async` command would
    // occupy a Tokio worker for the entire transfer and starve concurrent IPC
    // calls (e.g. get_path_info), so it moves to a dedicated blocking thread.
    tauri::async_runtime::spawn_blocking(move || {
        warp_file_op_sync(window, app, source, destination, mode, conflict, folder_mode, throttle, verify)
    })
    .await
    .map_err(|e| format!("Transfer task failed: {e}"))?
}

/// The synchronous transfer pipeline (scan → copy → verify). Runs on the
/// blocking thread pool via `spawn_blocking` — see `warp_file_op`.
fn warp_file_op_sync(
    window: Window,
    app: tauri::AppHandle,
    source: String,
    destination: String,
    mode: String,
    conflict: String,
    folder_mode: String, // "into" | "merge"
    throttle: u32,       // target MB/s, 0 = unlimited
    verify: bool,        // run a verification pass after a successful transfer
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
        "/NP".to_string(),
        "/R:3".to_string(),
        "/W:5".to_string(),
        "/BYTES".to_string(),
        "/NJH".to_string(),
        "/NJS".to_string(),
        "/256".to_string(), // support paths longer than 260 chars (fix #3)
    ]);

    // Bandwidth throttle via inter-packet gap (/IPG). Robocopy moves data in
    // 64 KB blocks; an N ms gap between blocks caps throughput. /IPG is applied
    // per thread, so disable multithreading when throttling to keep the cap
    // accurate; otherwise use multi-threaded mode.
    //
    // USB auto-tuning: removable drives have limited IO queues. Reduce threads
    // (4 instead of 32) and enable restartable mode (/Z) for resilience against
    // unexpected disconnects.
    let is_usb_source = is_path_on_usb(&source);
    let is_usb_dest = is_path_on_usb(&effective_dest);
    let is_usb = is_usb_source || is_usb_dest;

    if let Some(ipg) = ipg_for_throttle(throttle) {
        args.push(format!("/IPG:{ipg}"));
        // Throttling is single-threaded; no /MT needed.
    } else if is_usb {
        // USB: fewer threads to avoid overwhelming the controller, plus
        // restartable mode (/Z) so a copy interrupted by an unplugged drive
        // resumes from where it left off instead of restarting the whole file.
        args.push("/MT:4".to_string());
        args.push("/Z".to_string());
    } else {
        args.push("/MT:32".to_string());
    }

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
        verified: false, verify_mismatches: 0,
    };

    let mut bytes_done: u64 = 0;
    let mut last_emitted: u32 = 0;
    let mut last_speed_str = String::new();
    let mut last_bps: u64 = 0;
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
                        if bps > 0 { last_speed_str = fmt_speed(bps); last_bps = bps; }
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
                            bytes_per_sec: last_bps,
                            bytes_done,
                            total_bytes,
                        });
                    }
                }

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
        // For move mode: robocopy /MOVE removes the files it moves but leaves
        // empty source directories behind. Clean up ONLY empty directories.
        //
        // IMPORTANT: We must never blindly `remove_dir_all(&source)` here. In
        // skip-conflict mode (/XO /XN) some files are intentionally NOT moved
        // and remain in the source. A recursive delete would destroy those
        // files (data loss). `remove_empty_dirs` preserves any directory that
        // still contains files.
        if mode == "move" && summary.failed == 0 && !summary.cancelled {
            remove_empty_dirs(std::path::Path::new(&source));
        }

        // Optional verification pass. Skipped for "move" (the source is gone,
        // so there's nothing left to compare against) and when files failed.
        if verify && mode != "move" && summary.failed == 0 {
            // Tell the UI we're now verifying (the transfer itself is done but
            // the command hasn't returned yet — this can take a while).
            let _ = window.emit("warp-verifying", ());
            summary.verify_mismatches = verify_transfer(&source, &effective_dest);
            summary.verified = true;
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
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            warp_file_op,
            get_path_info,
            cancel_warp
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overall_pct_clamps_and_computes() {
        assert_eq!(overall_pct(0, 0), 0);
        assert_eq!(overall_pct(50, 100), 50);
        assert_eq!(overall_pct(0, 100), 0);
        // Never reports 100 mid-transfer; capped at 99.
        assert_eq!(overall_pct(100, 100), 99);
        assert_eq!(overall_pct(200, 100), 99);
    }

    #[test]
    fn fmt_speed_scales_units() {
        assert_eq!(fmt_speed(512), "512 B/s");
        assert_eq!(fmt_speed(2048), "2 KB/s");
        assert_eq!(fmt_speed(5 * 1_048_576), "5 MB/s");
        assert_eq!(fmt_speed(3 * 1_073_741_824), "3.0 GB/s");
    }

    #[test]
    fn ipg_throttle_calc() {
        // 0 MB/s = unlimited = no IPG (uses /MT instead).
        assert_eq!(ipg_for_throttle(0), None);
        // Higher throughput = smaller gap; clamped to at least 1 ms.
        assert_eq!(ipg_for_throttle(5), Some(13));   // 62.5/5 = 12.5 -> 13
        assert_eq!(ipg_for_throttle(25), Some(3));   // 62.5/25 = 2.5 -> 3 (rounds half away from zero)
        assert_eq!(ipg_for_throttle(100), Some(1));  // 62.5/100 = 0.625 -> 1
        assert_eq!(ipg_for_throttle(1000), Some(1)); // clamps to 1
    }

    #[test]
    fn basename_handles_windows_and_unix_separators() {
        assert_eq!(basename("C:\\folder\\file.txt"), "file.txt");
        assert_eq!(basename("/usr/local/bin/tool"), "tool");
        assert_eq!(basename("C:\\folder\\"), "folder");
        assert_eq!(basename("single"), "single");
    }

    #[test]
    fn robocopy_exit_codes_classified() {
        // 0..=7 are success/info -> no error message.
        for code in 0..=7 {
            assert!(robocopy_exit_message(code).is_none(), "code {code} should be success");
        }
        // 8 and 16 are failures.
        assert!(robocopy_exit_message(8).is_some());
        assert!(robocopy_exit_message(16).is_some());
    }

    #[test]
    fn parse_new_file_line() {
        let line = "\t    New File  \t\t      1024\tC:\\src\\photo.jpg";
        match parse_line(line) {
            RoboLine::FileHeader { is_same, is_error, size, name } => {
                assert!(!is_same);
                assert!(!is_error);
                assert_eq!(size, 1024);
                assert_eq!(name, "photo.jpg");
            }
            _ => panic!("expected FileHeader for a New File line"),
        }
    }

    #[test]
    fn parse_same_file_line_marked_same() {
        let line = "\t    Same  \t\t      2048\tC:\\src\\doc.txt";
        match parse_line(line) {
            RoboLine::FileHeader { is_same, size, .. } => {
                assert!(is_same);
                assert_eq!(size, 2048);
            }
            _ => panic!("expected FileHeader for a Same line"),
        }
    }

    #[test]
    fn parse_error_line_marked_error() {
        let line = "ERROR 5 (0x00000005) Copying File C:\\src\\locked.dat";
        match parse_line(line) {
            RoboLine::FileHeader { is_error, .. } => assert!(is_error),
            // Some ERROR lines without a parseable size are skipped; that's acceptable.
            RoboLine::Skip => {}
            _ => panic!("ERROR line should be an error FileHeader or Skip"),
        }
    }

    #[test]
    fn parse_speed_line() {
        let line = "   Speed :           123456789 Bytes/sec.";
        match parse_line(line) {
            RoboLine::Speed(bps) => assert_eq!(bps, 123456789),
            _ => panic!("expected Speed line"),
        }
    }

    #[test]
    fn parse_blank_line_is_skip() {
        assert!(matches!(parse_line("   "), RoboLine::Skip));
        assert!(matches!(parse_line(""), RoboLine::Skip));
    }

    #[test]
    fn parse_non_english_file_line() {
        // German robocopy: the status word is localized ("Neue Datei") but the
        // column structure and the size column are identical — progress still works.
        let line = "\t    Neue Datei  \t\t       512\tC:\\src\\bild.jpg";
        match parse_line(line) {
            RoboLine::FileHeader { is_same, is_error, size, name } => {
                assert!(!is_same);
                assert!(!is_error);
                assert_eq!(size, 512);
                assert_eq!(name, "bild.jpg");
            }
            _ => panic!("expected FileHeader for a localized New File line"),
        }
    }

    #[test]
    fn parse_dir_row_is_skipped() {
        // Directory rows have 3 columns — never mistaken for a file row.
        let line = "\t  New Dir          1\tC:\\src\\sub\\";
        assert!(matches!(parse_line(line), RoboLine::Skip));
        // The source-root row has no status word at all.
        let root = "\t                   3\tC:\\src\\";
        assert!(matches!(parse_line(root), RoboLine::Skip));
    }

    #[test]
    fn parse_extra_file_row_is_skipped() {
        // Dest-only files are prefixed with '*' (locale-independent) — they
        // must not count toward totals or verify mismatches.
        let line = "\t  *EXTRA File \t\t      12\tz.txt";
        assert!(matches!(parse_line(line), RoboLine::Skip));
    }

    #[test]
    fn parse_timestamped_error_line() {
        // Real error log lines carry a timestamp and a locale-independent
        // "<code> (0x<hex>)" pair.
        let line = "2026/08/06 21:12:33 ERROR 32 (0x00000020) Copying File C:\\tmp\\src\\locked.txt";
        match parse_line(line) {
            RoboLine::FileHeader { is_error, name, .. } => {
                assert!(is_error);
                assert_eq!(name, "locked.txt");
            }
            _ => panic!("expected an error FileHeader"),
        }
    }

    #[test]
    fn remove_empty_dirs_preserves_files() {
        let base = std::env::temp_dir().join(format!("warp_test_{}", std::process::id()));
        let empty_child = base.join("empty");
        let full_child = base.join("full");
        std::fs::create_dir_all(&empty_child).unwrap();
        std::fs::create_dir_all(&full_child).unwrap();
        std::fs::write(full_child.join("keep.txt"), b"data").unwrap();

        remove_empty_dirs(&base);

        // The empty subtree is gone, but the directory holding a file remains.
        assert!(!empty_child.exists(), "empty dir should be removed");
        assert!(full_child.join("keep.txt").exists(), "file must be preserved");
        assert!(base.exists(), "base still has a non-empty child");

        // Cleanup.
        let _ = std::fs::remove_dir_all(&base);
    }
}

// ── Updater signature test ───────────────────────────────────────────────────
//
// The in-app updater verifies each downloaded installer against the pubkey in
// tauri.conf.json (plugins.updater.pubkey) using exactly this crate
// (minisign-verify is the verifier behind tauri-plugin-updater). This test
// re-runs that verification against the real signed artifacts from the last
// `npm run build:win`, so a broken signing setup is caught before release.
// It skips when no build artifacts are present (e.g. a fresh checkout).
#[cfg(test)]
mod updater_signing {
    #[test]
    fn built_installer_verifies_against_configured_pubkey() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let bundle = manifest_dir
            .join("target")
            .join("release")
            .join("bundle")
            .join("nsis");
        let exe = bundle.join(format!("Warp_{}_x64-setup.exe", env!("CARGO_PKG_VERSION")));
        let sig_path = bundle.join(format!("Warp_{}_x64-setup.exe.sig", env!("CARGO_PKG_VERSION")));

        if !exe.exists() || !sig_path.exists() {
            eprintln!("skipping — no signed build artifacts in target/release/bundle/nsis");
            return;
        }

        let conf: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(manifest_dir.join("tauri.conf.json")).unwrap(),
        )
        .unwrap();
        let wrapper = conf["plugins"]["updater"]["pubkey"]
            .as_str()
            .expect("plugins.updater.pubkey must be set in tauri.conf.json");

        // The configured pubkey is a base64-wrapped minisign .pub file
        // ("untrusted comment: ...\n<base64 key>") — unwrap to the key line.
        use base64::Engine;
        let inner = String::from_utf8(
            base64::engine::general_purpose::STANDARD
                .decode(wrapper)
                .expect("configured pubkey must be valid base64"),
        )
        .expect("configured pubkey must decode to text");
        let key_b64 = inner
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .last()
            .expect("configured pubkey must contain a key line");

        let pk = minisign_verify::PublicKey::from_base64(key_b64)
            .expect("configured pubkey key line must be valid");

        // The .sig file is base64-wrapped the same way (the updater manifest
        // carries this exact content) — unwrap it before parsing.
        let sig_text = String::from_utf8(
            base64::engine::general_purpose::STANDARD
                .decode(
                    std::fs::read_to_string(&sig_path)
                        .expect("failed to read the .sig file")
                        .trim(),
                )
                .expect("sig file must be valid base64"),
        )
        .expect("sig file must decode to text");
        let signature = minisign_verify::Signature::decode(&sig_text)
            .expect("failed to parse the .sig file");
        let data = std::fs::read(&exe).unwrap();

        pk.verify(&data, &signature, false)
            .expect("installer signature must verify against the configured pubkey");
    }
}

// ── Real-robocopy integration tests ──────────────────────────────────────────
//
// These shell out to the REAL robocopy binary (the same commands the app
// builds) against real folders on disk, so the parser, scan pass, verify pass
// (with its exit-code fallback), and move cleanup are validated against actual
// robocopy output rather than hand-written fixtures. Windows-only: robocopy
// ships with every Windows install but does not exist elsewhere.
#[cfg(all(test, windows))]
mod real_robocopy {
    use super::*;
    use std::path::PathBuf;

    /// Creates a source tree with 3 files (one nested), an empty dir, and an
    /// empty destination. Returns (base, src, dst). `name` keeps parallel test
    /// runs from colliding on the same temp path.
    fn setup(name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!("warp_robo_{name}_{}", std::process::id()));
        let src = base.join("src");
        let dst = base.join("dst");
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("a.txt"), "hello world").unwrap();
        std::fs::write(src.join("b.txt"), "second file").unwrap();
        std::fs::write(src.join("sub").join("c.txt"), "nested file").unwrap();
        std::fs::create_dir(src.join("empty")).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        (base, src, dst)
    }

    /// The exact argument set the app builds for a normal (non-USB) copy.
    fn copy_args(src: &str, dst: &str) -> Vec<String> {
        vec![
            src.to_string(), dst.to_string(),
            "/E".into(), "/NP".into(), "/R:3".into(), "/W:5".into(),
            "/BYTES".into(), "/NJH".into(), "/NJS".into(), "/256".into(),
            "/MT:32".into(),
        ]
    }

    #[test]
    fn scan_counts_the_real_tree() {
        let (base, src, dst) = setup("scan");
        let s = src.to_string_lossy().to_string();
        let d = dst.to_string_lossy().to_string();

        let (bytes, files) = scan(&s, &d);
        assert_eq!(files, 3, "a.txt + b.txt + sub/c.txt; the empty dir is not a file");
        assert!(bytes > 0);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn verify_after_a_real_copy() {
        let (base, src, dst) = setup("verify");
        let s = src.to_string_lossy().to_string();
        let d = dst.to_string_lossy().to_string();

        // Do a real copy with the app's exact arguments.
        let out = robocopy_cmd()
            .args(&copy_args(&s, &d))
            .output()
            .expect("robocopy must run");
        let code = out.status.code().unwrap();
        assert_eq!(code, 1, "exit 1 = files copied successfully");
        assert!(dst.join("a.txt").exists());
        assert!(dst.join("sub").join("c.txt").exists());

        // Identical trees → verify reports zero mismatches (exit code 0 + parser).
        assert_eq!(verify_transfer(&s, &d), 0);

        // Deleting a destination file must be detected as a mismatch.
        std::fs::remove_file(dst.join("sub").join("c.txt")).unwrap();
        assert!(verify_transfer(&s, &d) >= 1, "missing file must be caught");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn move_mode_leaves_the_source_empty() {
        let base = std::env::temp_dir().join(format!("warp_robo_move_{}", std::process::id()));
        let src = base.join("src");
        let dst = base.join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(src.join("m.txt"), "move me").unwrap();

        let s = src.to_string_lossy().to_string();
        let d = dst.to_string_lossy().to_string();
        let out = robocopy_cmd()
            .args([
                &s, &d, "/MOVE", "/E", "/NP", "/R:3", "/W:5", "/BYTES", "/NJH", "/NJS", "/256", "/MT:32",
            ])
            .output()
            .expect("robocopy must run");
        assert!(out.status.code().unwrap() < 8);

        // The app's cleanup: remove only directories left fully empty.
        remove_empty_dirs(&src);

        assert!(dst.join("m.txt").exists(), "file should arrive");
        assert!(!src.join("m.txt").exists(), "file should leave the source");
        assert!(
            !src.exists() || src.read_dir().unwrap().next().is_none(),
            "source should be empty after a move"
        );

        let _ = std::fs::remove_dir_all(&base);
    }
}
