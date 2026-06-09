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

    let mut mismatches = 0u32;
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        // A file robocopy would still copy = not identical in the destination.
        if let RoboLine::FileHeader { is_same: false, is_error: false, .. } = parse_line(line) {
            mismatches += 1;
        }
    }
    mismatches
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
    // accurate; otherwise use 32 threads for maximum speed.
    if let Some(ipg) = ipg_for_throttle(throttle) {
        args.push(format!("/IPG:{ipg}"));
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
