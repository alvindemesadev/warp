#![allow(clippy::too_many_arguments, clippy::lines_filter_map_ok)]
mod backend;
mod engine_native;
mod engine_seq;
mod parser;
mod pool;
mod preflight;
mod progress;
mod shards;
mod trash;
mod verify;

pub use engine_seq::{parse_filter, warp_file_op_sync};
pub use preflight::{ensure_free_space, is_path_on_usb, to_long_path};
pub use progress::{fmt_speed, overall_pct};
pub use verify::verify_transfer;

use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use tauri::{Emitter, Manager, Window};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Registry key for the sequential engine's single robocopy child.
const SEQ_CHILD_ID: u64 = 1;

// — State ---------------------------------------------------------------------

/// Windows Job Object — kill-on-close: if Warp is `taskkill /F`'d, every robocopy child dies
/// even if Tauri's `Destroyed`/`Exit` handlers never ran. Created once per process.
#[cfg(windows)]
fn job_handle() -> Option<windows::Win32::Foundation::HANDLE> {
    use std::sync::OnceLock;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::JobObjects::{
        CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    struct SendHandle(HANDLE);
    unsafe impl Send for SendHandle {}
    unsafe impl Sync for SendHandle {}
    static JOB: OnceLock<Option<SendHandle>> = OnceLock::new();
    JOB.get_or_init(|| unsafe {
        let h = CreateJobObjectW(None, windows::core::PCWSTR::null()).ok()?;
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let ok = SetInformationJobObject(
            h,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const _,
            std::mem::size_of_val(&info) as u32,
        );
        if ok.is_err() {
            let _ = windows::Win32::Foundation::CloseHandle(h);
            return None;
        }
        Some(SendHandle(h))
    })
    .as_ref()
    .map(|sh| sh.0)
}

#[cfg(windows)]
fn assign_to_job(child: &std::process::Child) -> bool {
    use std::os::windows::io::AsRawHandle;
    if let Some(job) = job_handle() {
        let handle = windows::Win32::Foundation::HANDLE(child.as_raw_handle() as *mut _);
        unsafe { windows::Win32::System::JobObjects::AssignProcessToJobObject(job, handle).is_ok() }
    } else {
        false
    }
}

/// Shared transfer control: every live robocopy child (one for the sequential
/// engine, up to N for the parallel pool) registers here so cancellation,
/// pausing, and window-close cleanup can reach them all.
pub struct TransferControl {
    children: Mutex<HashMap<u64, std::process::Child>>,
    cancelled: AtomicBool,
    paused: AtomicBool,
}

impl Default for TransferControl {
    fn default() -> Self {
        Self {
            children: Mutex::new(HashMap::new()),
            cancelled: AtomicBool::new(false),
            paused: AtomicBool::new(false),
        }
    }
}

/// Poison-safe map lock — a panic in another thread must not brick cancel.
fn lock_children(
    m: &Mutex<HashMap<u64, std::process::Child>>,
) -> std::sync::MutexGuard<'_, HashMap<u64, std::process::Child>> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl TransferControl {
    fn reset_job(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
        self.paused.store(false, Ordering::SeqCst);
        lock_children(&self.children).clear();
    }
    fn register(&self, id: u64, child: std::process::Child) {
        #[cfg(windows)]
        {
            let _ = assign_to_job(&child);
        }
        lock_children(&self.children).insert(id, child);
    }
    /// Remove and return the child (ownership transfers back for `wait`).
    fn take(&self, id: u64) -> Option<std::process::Child> {
        lock_children(&self.children).remove(&id)
    }
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
    fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }
    fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::SeqCst);
    }
}

impl TransferControl {
    /// Kill every live child and clear the registry. Marks the job cancelled
    /// so worker read-loops stop immediately. Used by cancel, and by window-
    /// destroy/app-exit handlers — no code path may orphan robocopy.
    fn kill_all(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        let mut map = lock_children(&self.children);
        for (_, mut child) in map.drain() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

// — Types ---------------------------------------------------------------------

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
    /// Parallel engine only: live worker count (1 = sequential engine).
    #[serde(default)]
    pub active_workers: u32,
    #[serde(default)]
    pub shards_done: u32,
    #[serde(default)]
    pub shards_total: u32,
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
    /// Parallel engine only: workers actually used (1 = sequential).
    #[serde(default)]
    pub workers_used: u32,
    /// Files recovered by the automatic retry pass.
    #[serde(default)]
    pub retried_ok: u32,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PathMeta {
    pub files: u64,
    pub bytes: u64,
    pub is_file: bool,
    pub drive: String,   // e.g. "C:" for cross-drive detection
    pub removable: bool, // true if the drive is a removable/USB drive
}

pub(crate) fn hash_path(p: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(p.as_bytes());
    let result = hasher.finalize();
    format!("{:02x}{:02x}{:02x}{:02x}", result[0], result[1], result[2], result[3])
}

pub(crate) fn log_json(level: &str, event: &str, fields: serde_json::Value) {
    let path = std::env::temp_dir().join("warp.log");
    // Rotate at 5 MB
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > 5 * 1024 * 1024 {
            let rotated = std::env::temp_dir().join("warp.log.1");
            let _ = std::fs::remove_file(&rotated);
            let _ = std::fs::rename(&path, &rotated);
        }
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let mut obj = serde_json::json!({
            "ts": ts,
            "level": level,
            "event": event,
        });
        if let serde_json::Value::Object(map) = &mut obj {
            if let serde_json::Value::Object(extra) = fields {
                for (k, v) in extra {
                    map.insert(k, v);
                }
            }
        }
        let _ = writeln!(f, "{}", obj);
    }
}

pub(crate) fn log_event(msg: &str) {
    log_json("info", "event", serde_json::json!({ "msg": msg }));
}

// — Command factory -----------------------------------------------------------

pub(crate) fn robocopy_cmd() -> Command {
    let mut c = Command::new("robocopy");
    #[cfg(windows)]
    c.creation_flags(CREATE_NO_WINDOW);
    c.stdout(Stdio::piped()).stderr(Stdio::piped());
    c
}

// — Path info -----------------------------------------------------------------

#[tauri::command]
async fn get_path_info(path: String) -> Result<PathMeta, String> {
    // Walking a large tree can take a while — never block an async worker.
    tauri::async_runtime::spawn_blocking(move || get_path_info_sync(path))
        .await
        .map_err(|e| format!("Path scan task failed: {e}"))?
}

fn get_path_info_sync(path: String) -> Result<PathMeta, String> {
    let meta = std::fs::metadata(to_long_path(&path)).map_err(|e| e.to_string())?;

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

    let removable = !drive.is_empty() && preflight::is_removable_drive(&drive);

    Ok(PathMeta { files: count, bytes, is_file: false, drive, removable })
}

#[cfg(windows)]
fn walk_dir(dir: &str, count: &mut u64, bytes: &mut u64) {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        FindClose, FindExInfoBasic, FindExSearchNameMatch, FindFirstFileExW, FindNextFileW,
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FIND_FIRST_EX_LARGE_FETCH,
        WIN32_FIND_DATAW,
    };

    let mut stack = vec![to_long_path(dir).replace('/', "\\")];
    while let Some(current) = stack.pop() {
        let current_clean = current.trim_end_matches('\\');
        let pattern = format!("{current_clean}\\*");
        let pattern_utf16: Vec<u16> = pattern.encode_utf16().chain(std::iter::once(0)).collect();
        let mut find_data = WIN32_FIND_DATAW::default();

        let handle = unsafe {
            FindFirstFileExW(
                PCWSTR(pattern_utf16.as_ptr()),
                FindExInfoBasic,
                &mut find_data as *mut _ as *mut _,
                FindExSearchNameMatch,
                None,
                FIND_FIRST_EX_LARGE_FETCH,
            )
        };

        let handle = match handle {
            Ok(h) if !h.is_invalid() => h,
            _ => continue,
        };

        loop {
            let len = find_data
                .cFileName
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(find_data.cFileName.len());
            let name_slice = &find_data.cFileName[..len];

            // Ignore "." and ".."
            let is_dot = len == 1 && name_slice[0] == 0x2E;
            let is_dotdot = len == 2 && name_slice[0] == 0x2E && name_slice[1] == 0x2E;

            if !is_dot && !is_dotdot {
                let attrs = find_data.dwFileAttributes;
                let is_reparse = (attrs & FILE_ATTRIBUTE_REPARSE_POINT.0) != 0;

                if !is_reparse {
                    let is_dir = (attrs & FILE_ATTRIBUTE_DIRECTORY.0) != 0;
                    if is_dir {
                        let name_os = OsString::from_wide(name_slice);
                        let name_str = name_os.to_string_lossy();
                        let sub_path = format!("{current_clean}\\{name_str}");
                        stack.push(sub_path);
                    } else {
                        *count += 1;
                        let file_size = ((find_data.nFileSizeHigh as u64) << 32)
                            | (find_data.nFileSizeLow as u64);
                        *bytes += file_size;
                    }
                }
            }

            if unsafe { FindNextFileW(handle, &mut find_data).is_err() } {
                break;
            }
        }

        let _ = unsafe { FindClose(handle) };
    }
}

#[cfg(not(windows))]
fn walk_dir(dir: &str, count: &mut u64, bytes: &mut u64) {
    let mut stack = vec![std::path::PathBuf::from(to_long_path(dir))];
    while let Some(current) = stack.pop() {
        let rd = match std::fs::read_dir(&current) {
            Ok(rd) => rd,
            Err(e) => {
                log_json(
                    "warn",
                    "walk_dir",
                    serde_json::json!({ "path_hash": hash_path(&current.display().to_string()), "error": e.to_string() }),
                );
                continue;
            }
        };
        for entry in rd.flatten() {
            if let Ok(ft) = entry.file_type() {
                if ft.is_symlink() {
                    continue;
                }
                if ft.is_dir() {
                    stack.push(entry.path());
                } else if ft.is_file() {
                    if let Ok(m) = entry.metadata() {
                        *count += 1;
                        *bytes += m.len();
                    }
                }
            }
        }
    }
}

/// (bytes, files) under `dir` — shared with the shard partitioner.
pub fn dir_stats(dir: &str) -> (u64, u64) {
    let mut count = 0u64;
    let mut bytes = 0u64;
    walk_dir(dir, &mut count, &mut bytes);
    (bytes, count)
}

/// Recursively remove empty directories starting from `dir`, bottom-up.
/// A directory is removed only if it ends up empty after its empty children
/// are removed. Any directory that still contains files is left untouched.
/// This is used after a `/MOVE` to clean up leftover empty folders WITHOUT
/// risking deletion of files that were skipped and intentionally left behind.
pub(crate) fn remove_empty_dirs(dir: &std::path::Path) -> bool {
    // Handle long paths via \\?\ prefix
    let dir_str = dir.to_string_lossy().to_string();
    let long = std::path::PathBuf::from(to_long_path(&dir_str));
    let dir = long.as_path();
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
        log_json(
            "warn",
            "remove_empty_dirs",
            serde_json::json!({ "path_hash": hash_path(&dir.display().to_string()) }),
        );
        // Couldn't read the directory; don't attempt to remove it.
        return false;
    }

    if is_empty {
        std::fs::remove_dir(dir).is_ok()
    } else {
        false
    }
}

// — Cancel / pause ------------------------------------------------------------

/// Kill every active robocopy child and mark the job cancelled. The cancel
/// button AND the window-destroy/app-exit handlers funnel here.
fn kill_active_children(app: &tauri::AppHandle) {
    app.state::<TransferControl>().kill_all();
}

#[tauri::command]
async fn cancel_warp(app: tauri::AppHandle) -> Result<(), String> {
    let control = app.state::<TransferControl>();
    control.set_paused(false);
    control.kill_all();
    Ok(())
}

/// Pause = dispatch gate. In-flight shards finish their current robocopy; no
/// new shards are dispatched until resumed. Honest UI copy reflects this.
#[tauri::command]
async fn pause_warp(app: tauri::AppHandle, paused: bool) -> Result<(), String> {
    if !paused {
        // Resuming must never resurrect a cancelled job's flags.
        let control = app.state::<TransferControl>();
        if !control.is_cancelled() {
            control.set_paused(false);
        }
    } else {
        app.state::<TransferControl>().set_paused(true);
    }
    Ok(())
}

#[tauri::command]
async fn undo_last() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(trash::undo_last).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn check_health(path: String) -> Result<Option<f64>, String> {
    Ok(preflight::health_mbps(&path))
}

fn basename(path: &str) -> String {
    path.replace('\\', "/").split('/').rfind(|s| !s.is_empty()).unwrap_or(path).to_string()
}

/// Translate robocopy exit codes to human-readable messages.
pub(crate) fn robocopy_exit_message(code: i32) -> Option<String> {
    match code {
        0..=7 => None, // success
        8 => Some(
            "Some files or directories could not be copied — check: files in use (close Outlook/Excel/DB), access denied (try admin), disk full, path too long, or network disconnect. See per-file errors below.".to_string(),
        ),
        16 => Some("Robocopy did not copy any files — serious error. Check source/destination paths, permissions, and that the drive is online.".to_string()),
        _ => {
            if code & 8 != 0 {
                if code & 16 != 0 {
                    Some(format!("Transfer failed (exit code {}: copy errors + serious error) — possible disk full, access denied, or share offline. Check per-file errors.", code))
                } else {
                    Some(format!("Transfer failed (exit code {}: some files failed) — check for locked files (0x20), access denied (0x5), or disk full (0x70).", code))
                }
            } else {
                Some(format!("Transfer failed with exit code {} (info bits 1/2/4 only, but treated as failure).", code))
            }
        }
    }
}

// — Parser --------------------------------------------------------------------

pub(crate) enum RoboLine {
    FileHeader { is_same: bool, is_error: bool, size: u64, name: String },
    Extra { size: u64, name: String },
    Percent(f64),
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
pub(crate) fn parse_line(raw: &str) -> RoboLine {
    let t = raw.trim();
    if t.is_empty() {
        return RoboLine::Skip;
    }

    // Speed line (best-effort — the "Bytes/sec" label is localized too, but
    // live speed is also computed from file sizes, so this only helps the
    // very first second of a transfer).
    if t.to_lowercase().contains("bytes/sec") {
        for tok in t.split_whitespace() {
            if let Ok(bps) = tok.replace(',', "").parse::<u64>() {
                if bps > 1000 {
                    return RoboLine::Speed(bps);
                }
            }
        }
        return RoboLine::Skip;
    }

    // Percent progress for large files, e.g. " 12.3%  New File  5.0g  C:\big.dat" or "  100.0%"
    // Must come before error/file parsing so "12.3%" isn't mistaken for a file.
    if t.contains('%') {
        for tok in t.split_whitespace() {
            if tok.ends_with('%') {
                if let Ok(p) = tok.trim_end_matches('%').replace(',', "").parse::<f64>() {
                    if (0.0..=100.0).contains(&p) {
                        return RoboLine::Percent(p);
                    }
                }
            }
        }
    }

    // Error log lines, e.g. "2026/08/06 21:12:33 ERROR 32 (0x00000020) Copying
    // File C:\...". The "<decimal> (0x<hex>)" pair is rendered the same in
    // every locale, unlike the "ERROR" word itself.
    {
        let toks: Vec<&str> = t.split_whitespace().collect();
        for (i, tok) in toks.iter().enumerate() {
            if tok.parse::<u32>().is_err() {
                continue;
            }
            let Some(hex) = toks.get(i + 1) else { break };
            let is_hex_code = hex.starts_with("(0x")
                && hex.ends_with(')')
                && hex.len() > 4
                && hex[3..hex.len() - 1].chars().all(|c| c.is_ascii_hexdigit());
            if is_hex_code {
                let base = basename(&toks[i + 2..].join(" "));
                let hint = match *tok {
                    "32" => " — file in use (close the file) ",
                    "33" => " — file in use (close the file) ",
                    "5" => " — access denied ",
                    "2" => " — file not found ",
                    "3" => " — path not found ",
                    "80" => " — file already exists ",
                    "112" => " — disk full ",
                    _ => " ",
                };
                let name = if hint.trim().is_empty() {
                    base.clone()
                } else {
                    format!("{}{}(error {} {})", base, hint, tok, hex)
                };
                return RoboLine::FileHeader { is_same: false, is_error: true, size: 0, name };
            }
        }
    }

    // File-list rows are tab-delimited. IMPORTANT: split `raw` (not `t`) — the
    // leading tab is what keeps column 0 empty.
    //   File rows:  ["", "New File", "", "1024", "path"]   (5 columns)
    //   Dir rows:   ["", "New Dir  1", "path"]             (3 columns, skipped)
    //   Extra rows: ["", "*EXTRA File", "", "12", "path"] (dest-only, used for Sync deletes)
    let cols: Vec<&str> = raw.split('\t').collect();
    if cols.len() >= 5 {
        let status = cols[1].trim();
        let path = cols[4..].join(" ").trim().to_string();
        if let Ok(size) = cols[3].trim().parse::<u64>() {
            if !status.is_empty() && !path.is_empty() {
                if status.starts_with('*') {
                    return RoboLine::Extra { size, name: basename(&path) };
                }
                let is_same = status.eq_ignore_ascii_case("Same");
                let is_error = status.eq_ignore_ascii_case("ERROR");
                return RoboLine::FileHeader { is_same, is_error, size, name: basename(&path) };
            }
        }
    }

    RoboLine::Skip
}

// — Main transfer command -----------------------------------------------------

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
    workers: Option<u8>, // parallel workers: None/0 = Auto, 2..=8 explicit
    filter: Option<String>,
) -> Result<WarpSummary, String> {
    // The whole pipeline (scan pass + streaming robocopy output) is synchronous
    // and can run for a long time. Running it inside an `async` command would
    // occupy a Tokio worker for the entire transfer and starve concurrent IPC
    // calls (e.g. get_path_info), so it moves to a dedicated blocking thread.
    tauri::async_runtime::spawn_blocking(move || {
        run_transfer(
            window,
            app,
            source,
            destination,
            mode,
            conflict,
            folder_mode,
            throttle,
            verify,
            workers,
            filter,
        )
    })
    .await
    .map_err(|e| format!("Transfer task failed: {e}"))?
}

// — Engine selection ----------------------------------------------------------

/// Cheap pre-partition gate. Explicit worker requests (>1) bypass the job-size
/// heuristics but never the correctness gates.
/// Sync now uses two-phase parallel (delete *EXTRA in parallel -> then copy
/// in parallel, strictly sequential — 8 delete -> 8 copy). Only throttle
/// remains a hard single-process gate (IPG per-process).
fn should_attempt_parallel(
    requested: Option<u8>,
    mode: &str,
    throttle: u32,
    total_files: u64,
    total_bytes: u64,
    top_dirs: usize,
) -> bool {
    if mode == "sync" || throttle > 0 || top_dirs < 2 {
        return false;
    }
    match requested {
        Some(w) if w > 1 => true,
        _ => total_files >= 400 || total_bytes >= 64 * 1024 * 1024,
    }
}

/// Orchestrator: shared preflights, then dispatch to the sequential engine or
/// the parallel shard pool. Runs on a blocking thread (see `warp_file_op`).
#[allow(clippy::too_many_arguments)]
fn run_transfer(
    window: Window,
    app: tauri::AppHandle,
    source: String,
    destination: String,
    mode: String,
    conflict: String,
    folder_mode: String,
    throttle: u32,
    verify: bool,
    workers: Option<u8>,
    filter: Option<String>,
) -> Result<WarpSummary, String> {
    let control = app.state::<TransferControl>();
    control.reset_job();

    let effective_dest = preflight::resolve_effective_dest(&source, &destination, &folder_mode);

    log_json(
        "info",
        "start",
        serde_json::json!({
            "src_hash": hash_path(&source),
            "dst_hash": hash_path(&effective_dest),
            "mode": mode,
            "conflict": conflict,
            "folder": folder_mode,
            "throttle": throttle,
            "verify": verify,
            "workers": workers
        }),
    );

    preflight::check_overlap(&source, &effective_dest)?;
    preflight::check_network_dest(&effective_dest)?;
    preflight::check_fat32_source(&source, &effective_dest)?;

    let (quick_bytes, quick_files) = dir_stats(&source);
    let top_dirs = shards::top_level_dir_count(&source);

    let is_network = source.starts_with(r"\\") || effective_dest.starts_with(r"\\");
    let is_usb = preflight::is_path_on_usb(&source) || preflight::is_path_on_usb(&effective_dest);

    let mut engine_workers =
        if should_attempt_parallel(workers, &mode, throttle, quick_files, quick_bytes, top_dirs) {
            pool::resolve_workers_for(
                workers.unwrap_or(0),
                &mode,
                throttle,
                usize::MAX, // shard count unknown pre-partition; hard gates re-checked inside
                quick_files,
                quick_bytes,
                is_usb,
                is_network,
            )
        } else {
            1
        };

    // Disk health: if dest is a slow SD/SMR (<10 MB/s) and Auto, cap at 2 lanes
    if let Some(mbps) = preflight::health_mbps(&effective_dest) {
        if mbps < 10.0 && workers.unwrap_or(0) == 0 && engine_workers > 2 {
            log_json(
                "info",
                "slow_drive",
                serde_json::json!({ "dst_hash": hash_path(&effective_dest), "mbps": mbps, "workers": 2 }),
            );
            let _ = window
                .emit("warp-health-warning", serde_json::json!({ "slow": true, "mbps": mbps }));
            engine_workers = 2;
        }
    }

    let summary =
        if engine_workers > 1 && filter.is_none() && mode != "sync" && throttle == 0 && !is_network
        {
            engine_native::warp_file_op_native(
                window,
                &control,
                source,
                destination,
                effective_dest,
                mode,
                conflict,
                verify,
                engine_workers,
                quick_bytes,
                quick_files as u32,
            )?
        } else if engine_workers > 1 {
            transfer_parallel(
                window,
                &control,
                &source,
                &destination,
                &effective_dest,
                &mode,
                &conflict,
                throttle,
                verify,
                engine_workers,
                filter,
                quick_bytes,
                quick_files,
            )?
        } else {
            engine_seq::warp_file_op_sync(
                window,
                &control,
                source,
                destination,
                effective_dest,
                mode,
                conflict,
                throttle,
                verify,
                filter,
                quick_bytes,
                quick_files as u32,
                workers,
            )?
        };

    control.reset_job(); // leave flags clean for the next job
    Ok(summary)
}

// — Parallel engine -----------------------------------------------------------
//
// Coordinator + worker pool over disjoint directory shards (see shards.rs).
// Safety model:
//   - every shard owns a disjoint source subtree and therefore a disjoint
//     destination subtree — file conflicts between workers are impossible by
//     construction, no path-level locking needed
//   - every robocopy child registers in TransferControl, so cancel / pause /
//     window-close reach all workers exactly like the sequential engine
//   - sync (/MIR) and throttled transfers never enter this engine (hard gates
//     in `should_attempt_parallel` / `resolve_workers_for`)

/// Event sink so shard execution stays testable without a Tauri Window.
trait ShardSink: Send + Sync {
    fn progress(&self, p: &WarpProgress);
    fn error_line(&self, s: &str);
}

struct WindowSink {
    window: Window,
    shards_total: u32,
    /// Workers currently running a shard — decays as the queue drains.
    live_workers: std::sync::Arc<std::sync::atomic::AtomicU32>,
    /// Shards fully completed.
    done_shards: std::sync::Arc<std::sync::atomic::AtomicU32>,
}
impl ShardSink for WindowSink {
    fn progress(&self, p: &WarpProgress) {
        use std::sync::atomic::Ordering;
        let mut p = p.clone();
        p.active_workers = self.live_workers.load(Ordering::SeqCst);
        p.shards_total = self.shards_total;
        p.shards_done = self.done_shards.load(Ordering::SeqCst);
        let _ = self.window.emit("warp-progress", p);
    }
    fn error_line(&self, s: &str) {
        let _ = self.window.emit("warp-error", s.to_string());
    }
}

/// Poison-safe lock helper shared by the coordinator's structures.
fn lock_ok<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Spawn one robocopy for `args`, register it under `id`, stream its output
/// into the optional shared tracker + local counters, wait for exit.
fn run_shard(
    control: &TransferControl,
    id: u64,
    args: &[String],
    tracker: Option<&Mutex<pool::Tracker>>,
    sink: std::sync::Arc<dyn ShardSink>,
) -> Result<pool::ShardOutcome, String> {
    let mut child =
        robocopy_cmd().args(args).spawn().map_err(|e| format!("Failed to start robocopy: {e}"))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    control.register(id, child);

    // Stderr reader — surface lines like the sequential engine does.
    if let Some(stderr) = stderr {
        let snk = sink.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().flatten() {
                let t = line.trim().to_string();
                if !t.is_empty() {
                    snk.error_line(&t);
                    log_event(&format!("robocopy stderr: {}", t));
                }
            }
        });
    }

    let mut local = pool::LocalCounters::default();
    if let Some(out) = stdout {
        let snk_p = sink.clone();
        let snk_e = sink.clone();
        pool::consume_stream(
            BufReader::new(out),
            &|| control.is_cancelled(),
            tracker,
            &mut local,
            move |p| snk_p.progress(&p),
            move |e| snk_e.error_line(&e),
        );
    }

    let (exit_code, had_exit_code) = match control.take(id) {
        Some(ref mut c) => match c.wait() {
            Ok(status) => match status.code() {
                Some(v) => (v, true),
                None => (-1, false), // killed without exit code (cancel/close)
            },
            Err(_) => (-1, false),
        },
        // Removed from the registry by cancel/kill_all before we took it back.
        None => (-1, false),
    };

    Ok(pool::ShardOutcome::from_local(id, &local, exit_code, had_exit_code))
}

/// File-chunk shard via robocopy (Plan B) — keeps robocopy's /MT and buffering.
/// Uses `robocopy src dst file1 file2 ...` so we stay on the Windows engine for all
/// COPY/MOVE/SYNC. Falls back to direct copy only if arg list would overflow.
fn run_file_chunk_shard(
    control: &TransferControl,
    shard: &shards::Shard,
    tracker: Option<&Mutex<pool::Tracker>>,
    sink: std::sync::Arc<dyn ShardSink>,
    skip_conflict: bool,
) -> Result<pool::ShardOutcome, String> {
    let files = shard.chunk_files.as_ref().ok_or("not a file chunk")?;
    // Windows cmd limit 8191 — estimate 20 chars per file + overhead. If too many, fallback to direct.
    let est_len: usize =
        shard.src.len() + shard.dst.len() + files.iter().map(|f| f.len() + 1).sum::<usize>() + 200;
    if est_len > 7000 {
        // Fallback: direct copy chunk-by-chunk (rare, only for >~300 files per chunk)
        let mut local = pool::LocalCounters::default();
        let src_dir = std::path::Path::new(&shard.src);
        let dst_dir = std::path::Path::new(&shard.dst);
        let _ = std::fs::create_dir_all(to_long_path(&shard.dst));
        for name in files {
            while control.is_paused() && !control.is_cancelled() {
                std::thread::sleep(std::time::Duration::from_millis(80));
            }
            if control.is_cancelled() {
                break;
            }
            let src_path = src_dir.join(name);
            let dst_path = dst_dir.join(name);
            let src_long = to_long_path(&src_path.to_string_lossy());
            let dst_long = to_long_path(&dst_path.to_string_lossy());
            if skip_conflict && std::fs::metadata(&dst_long).is_ok() {
                local.skipped += 1;
                local.seen += 1;
                if let Some(t) = tracker.as_ref() {
                    let mut g = t.lock().unwrap_or_else(|e| e.into_inner());
                    let line = RoboLine::FileHeader {
                        is_same: true,
                        is_error: false,
                        size: 0,
                        name: name.clone(),
                    };
                    if let Some(p) = g.ingest(&line, Instant::now()) {
                        drop(g);
                        sink.progress(&p);
                    }
                }
                continue;
            }
            let meta = match std::fs::metadata(&src_long) {
                Ok(m) => m,
                Err(e) => {
                    local.failed += 1;
                    local.seen += 1;
                    sink.error_line(&format!("{} — file not found: {}", name, e));
                    if let Some(t) = tracker.as_ref() {
                        let mut g = t.lock().unwrap_or_else(|e| e.into_inner());
                        let line = RoboLine::FileHeader {
                            is_same: false,
                            is_error: true,
                            size: 0,
                            name: name.clone(),
                        };
                        if let Some(p) = g.ingest(&line, Instant::now()) {
                            drop(g);
                            sink.progress(&p);
                        }
                    }
                    continue;
                }
            };
            let size = meta.len();
            match std::fs::copy(&src_long, &dst_long) {
                Ok(_) => {
                    local.transferred += 1;
                    local.seen += 1;
                    local.counted_bytes += size;
                    if let Some(t) = tracker.as_ref() {
                        let mut g = t.lock().unwrap_or_else(|e| e.into_inner());
                        let line = RoboLine::FileHeader {
                            is_same: false,
                            is_error: false,
                            size,
                            name: name.clone(),
                        };
                        if let Some(p) = g.ingest(&line, Instant::now()) {
                            drop(g);
                            sink.progress(&p);
                        }
                    }
                }
                Err(e) => {
                    local.failed += 1;
                    local.seen += 1;
                    sink.error_line(&format!("{} — copy failed: {}", name, e));
                    if let Some(t) = tracker.as_ref() {
                        let mut g = t.lock().unwrap_or_else(|e| e.into_inner());
                        let line = RoboLine::FileHeader {
                            is_same: false,
                            is_error: true,
                            size: 0,
                            name: name.clone(),
                        };
                        if let Some(p) = g.ingest(&line, Instant::now()) {
                            drop(g);
                            sink.progress(&p);
                        }
                    }
                }
            }
        }
        let exit_code = if local.failed > 0 { 8 } else { 0 };
        return Ok(pool::ShardOutcome::from_local(shard.id, &local, exit_code, true));
    }
    // Fast path: robocopy with file list + /MT:8 (keeps Windows buffering, COPY:DAT, retries)
    let mut args = vec![shard.src.clone(), shard.dst.clone()];
    // Add file names (robocopy source destination [file ...])
    for f in files {
        // Basic sanitization: skip overly long or tricky names
        if f.len() > 100 || f.contains("..") || f.contains('\\') || f.contains('/') {
            continue;
        }
        args.push(f.clone());
    }
    if skip_conflict {
        args.push("/XO".to_string());
        args.push("/XN".to_string());
    }
    args.extend([
        "/NP".to_string(),
        "/R:3".to_string(),
        "/W:5".to_string(),
        "/BYTES".to_string(),
        "/NJH".to_string(),
        "/NJS".to_string(),
        "/256".to_string(),
        "/XJ".to_string(),
        "/XJD".to_string(),
        "/COPY:DAT".to_string(),
        "/MT:8".to_string(),
    ]);
    // Reuse the standard robocopy runner so we get parse_line, Tracker, and JobObject handling
    let mut child =
        robocopy_cmd().args(&args).spawn().map_err(|e| format!("Failed to start robocopy: {e}"))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    control.register(shard.id, child);
    if let Some(stderr) = stderr {
        let snk = sink.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().flatten() {
                let t = line.trim().to_string();
                if !t.is_empty() {
                    snk.error_line(&t);
                    log_event(&format!("robocopy stderr: {}", t));
                }
            }
        });
    }
    let mut local = pool::LocalCounters::default();
    if let Some(out) = stdout {
        let snk_p = sink.clone();
        let snk_e = sink.clone();
        pool::consume_stream(
            BufReader::new(out),
            &|| control.is_cancelled(),
            tracker,
            &mut local,
            move |p| snk_p.progress(&p),
            move |e| snk_e.error_line(&e),
        );
    }
    let (exit_code, had_exit_code) = match control.take(shard.id) {
        Some(ref mut c) => match c.wait() {
            Ok(s) => match s.code() {
                Some(v) => (v, true),
                None => (-1, false),
            },
            Err(_) => (-1, false),
        },
        None => (-1, false),
    };
    Ok(pool::ShardOutcome::from_local(shard.id, &local, exit_code, had_exit_code))
}

// — Sync two-phase helpers (delete -> copy, same workers reused) -------------

struct ExtraFile {
    full_path: String,
    size: u64,
    name: String,
}

fn matches_filter(name: &str, patterns: &[String]) -> bool {
    let lower = name.to_lowercase();
    for pat in patterns {
        let p = pat.to_lowercase();
        if p.contains('*') {
            let stripped = p.replace('*', "");
            if stripped.is_empty() {
                return true;
            }
            if p.starts_with('*') && p.ends_with('*') {
                if lower.contains(&stripped) {
                    return true;
                }
            } else if p.starts_with('*') {
                if lower.ends_with(&stripped) {
                    return true;
                }
            } else if p.ends_with('*') {
                if lower.starts_with(&stripped) {
                    return true;
                }
            } else if lower.contains(&stripped) {
                return true;
            }
        } else if lower == p {
            return true;
        }
    }
    false
}

fn clean_long_prefix(p: &str) -> String {
    let mut s = p.to_string();
    if s.starts_with(r"\\?\") {
        s = s[4..].to_string();
        if s.starts_with(r"UNC\") {
            s = format!(r"\\{}", &s[4..]);
        }
    }
    s
}

fn collect_extra_files(source: &str, effective_dest: &str, filter: Option<&str>) -> Vec<ExtraFile> {
    let patterns = parse_filter(filter);
    if std::fs::metadata(to_long_path(effective_dest)).is_err() {
        return Vec::new();
    }
    let src_root = std::path::Path::new(source);
    let dest_clean = effective_dest.trim_end_matches('\\').to_lowercase();
    let mut out = Vec::new();
    let mut stack = vec![std::path::PathBuf::from(to_long_path(effective_dest))];
    while let Some(current) = stack.pop() {
        let rd = match std::fs::read_dir(&current) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for entry in rd.flatten() {
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_symlink() {
                continue;
            }
            let path = entry.path();
            let path_str = clean_long_prefix(&path.to_string_lossy());
            let name = entry.file_name().to_string_lossy().to_string();
            if !patterns.is_empty() && matches_filter(&name, &patterns) {
                continue;
            }
            if ft.is_file() {
                let rel_str = if path_str.to_lowercase().starts_with(&dest_clean) {
                    path_str[effective_dest.trim_end_matches('\\').len()..]
                        .trim_start_matches('\\')
                        .to_string()
                } else {
                    // Fallback to Path strip_prefix
                    match path.strip_prefix(std::path::Path::new(effective_dest)) {
                        Ok(r) => r.to_string_lossy().to_string(),
                        Err(_) => continue,
                    }
                };
                if rel_str.is_empty() {
                    continue;
                }
                let src_counterpart = src_root.join(&rel_str);
                let src_long = to_long_path(&src_counterpart.to_string_lossy());
                if std::fs::metadata(&src_long).is_err() {
                    let sz = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    out.push(ExtraFile {
                        full_path: path.to_string_lossy().to_string(),
                        size: sz,
                        name,
                    });
                }
            } else if ft.is_dir() {
                stack.push(path);
            }
        }
    }
    out
}

fn cleanup_extra_empty_dirs(source: &str, effective_dest: &str, filter: Option<&str>) -> u32 {
    let patterns = parse_filter(filter);
    let dest_root_clean = effective_dest.trim_end_matches('\\').to_lowercase();
    let src_root = std::path::Path::new(source);
    if std::fs::metadata(to_long_path(effective_dest)).is_err() {
        return 0;
    }
    // Collect all dirs bottom-up
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();
    let mut stack = vec![std::path::PathBuf::from(to_long_path(effective_dest))];
    while let Some(cur) = stack.pop() {
        let rd = match std::fs::read_dir(&cur) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for entry in rd.flatten() {
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_symlink() {
                continue;
            }
            if ft.is_dir() {
                let p = entry.path();
                dirs.push(p.clone());
                stack.push(p);
            }
        }
    }
    // Deepest first (longest path)
    dirs.sort_by_key(|b| std::cmp::Reverse(b.to_string_lossy().len()));
    let mut removed = 0u32;
    for dir in dirs {
        let dir_str_clean = clean_long_prefix(&dir.to_string_lossy());
        if dir_str_clean
            .trim_end_matches('\\')
            .eq_ignore_ascii_case(effective_dest.trim_end_matches('\\'))
        {
            continue;
        }
        let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !patterns.is_empty() && matches_filter(name, &patterns) {
            continue;
        }
        let rel_str = if dir_str_clean.to_lowercase().starts_with(&dest_root_clean) {
            dir_str_clean[effective_dest.trim_end_matches('\\').len()..]
                .trim_start_matches('\\')
                .to_string()
        } else {
            match dir.strip_prefix(std::path::Path::new(effective_dest)) {
                Ok(r) => r.to_string_lossy().to_string(),
                Err(_) => continue,
            }
        };
        if rel_str.is_empty() {
            continue;
        }
        let src_counterpart = src_root.join(&rel_str);
        let src_long = to_long_path(&src_counterpart.to_string_lossy());
        if std::fs::metadata(&src_long).is_ok() {
            continue; // dir exists in source — keep it even if empty
        }
        // Extra dir — remove if empty
        let is_empty = match std::fs::read_dir(&dir) {
            Ok(mut rd) => rd.next().is_none(),
            Err(_) => false,
        };
        if is_empty && std::fs::remove_dir(&dir).is_ok() {
            removed += 1;
        }
    }
    removed
}

#[allow(clippy::too_many_arguments)]
fn transfer_parallel(
    window: Window,
    control: &TransferControl,
    source: &str,
    destination: &str,
    effective_dest: &str,
    mode: &str,
    conflict: &str,
    throttle: u32,
    verify: bool,
    workers_requested: usize,
    filter: Option<String>,
    total_bytes: u64,
    total_files: u64,
) -> Result<WarpSummary, String> {
    let start = Instant::now();

    let shard_list = shards::partition_balanced_with_stats(
        source,
        effective_dest,
        workers_requested,
        total_bytes,
    );
    if shard_list.len() < 2 && !shard_list.iter().any(|s| s.chunk_files.is_some()) {
        return warp_file_op_sync(
            window,
            control,
            source.to_string(),
            destination.to_string(),
            effective_dest.to_string(),
            mode.to_string(),
            conflict.to_string(),
            throttle,
            verify,
            filter,
            total_bytes,
            total_files as u32,
            Some(workers_requested as u8),
        );
    }

    let workers = workers_requested.min(shard_list.len()).max(1);

    // — Two-phase Sync: collect *EXTRA (dest-only) for delete phase ---------
    let is_sync = mode == "sync";
    let source_bytes: u64 = shard_list.iter().map(|s| s.est_bytes).sum();
    let source_files: u32 =
        shard_list.iter().map(|s| s.est_files.min(u32::MAX as u64) as u32).sum();
    let mut extra_files_vec: Vec<ExtraFile> = Vec::new();
    let mut extra_bytes: u64 = 0;
    let mut extra_count: u32 = 0;
    if is_sync {
        extra_files_vec = collect_extra_files(source, effective_dest, filter.as_deref());
        extra_bytes = extra_files_vec.iter().map(|e| e.size).sum();
        extra_count = extra_files_vec.len() as u32;
        if extra_count > 0 {
            log_event(&format!(
                "sync delete phase: {} extra files, {} bytes",
                extra_count, extra_bytes
            ));
        }
    }
    let total_bytes = source_bytes.saturating_add(extra_bytes);
    let total_files_scan = source_files.saturating_add(extra_count);
    // Free-space needs only source bytes (deletes free space); using total would over-block
    ensure_free_space(destination, effective_dest, source_bytes)?;

    let shards_total = shard_list.len() as u32;
    let usb = is_path_on_usb(source) || is_path_on_usb(effective_dest);
    let move_mode = mode == "move";
    let skip_conflict = conflict == "skip";

    log_event(&format!(
        "parallel: {} shards, {} workers, {} bytes est (source {} + extra {}), mode={}",
        shards_total, workers, total_bytes, source_bytes, extra_bytes, mode
    ));

    // Metadata kept aside BEFORE the queue consumes the shards — retries need
    // each failed shard's src/dst/root_only/chunk_files back.
    type ShardMeta = (String, String, bool, Option<Vec<String>>);
    let shard_meta: HashMap<u64, ShardMeta> = shard_list
        .iter()
        .map(|s| (s.id, (s.src.clone(), s.dst.clone(), s.root_only, s.chunk_files.clone())))
        .collect();

    let tracker =
        Mutex::new(pool::Tracker::new(total_bytes, total_files_scan, total_bytes == 0, false));
    let queue: Mutex<VecDeque<shards::Shard>> = Mutex::new(shard_list.into_iter().collect());
    let outcomes: Mutex<Vec<pool::ShardOutcome>> = Mutex::new(Vec::new());
    use std::sync::atomic::AtomicU32;
    let live_workers = std::sync::Arc::new(AtomicU32::new(0));
    let done_shards = std::sync::Arc::new(AtomicU32::new(0));

    let sink: std::sync::Arc<dyn ShardSink> = std::sync::Arc::new(WindowSink {
        window: window.clone(),
        shards_total,
        live_workers: live_workers.clone(),
        done_shards: done_shards.clone(),
    });

    let mt_per_worker: u32 = if usb { 4 } else { 8 };

    // — Sync Phase 1: parallel delete of *EXTRA (strictly before any copy) ---
    // Reuses the same worker count (8 delete -> 8 copy, never 4+4 concurrent).
    // Tracker is shared so progress is continuous; live_workers shows 8 during
    // both phases, done_shards only tracks copy shards.
    let mut sync_delete_transferred: u32 = 0;
    let mut sync_delete_failed: u32 = 0;
    if is_sync && !extra_files_vec.is_empty() {
        use std::sync::atomic::Ordering;
        let dq: Mutex<VecDeque<ExtraFile>> = Mutex::new(extra_files_vec.into_iter().collect());
        let failed = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let transferred = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let bytes = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        log_event(&format!(
            "sync phase 1: deleting {} extra files with {} workers",
            extra_count, workers
        ));
        std::thread::scope(|scope| {
            for _ in 0..workers {
                scope.spawn(|| {
                    live_workers.fetch_add(1, Ordering::SeqCst);
                    loop {
                        while control.is_paused() && !control.is_cancelled() {
                            std::thread::sleep(std::time::Duration::from_millis(80));
                        }
                        if control.is_cancelled() {
                            break;
                        }
                        let entry = lock_ok(&dq).pop_front();
                        let Some(entry) = entry else { break };
                        let long = to_long_path(&entry.full_path);
                        let res =
                            std::fs::remove_file(&long).or_else(|_| std::fs::remove_dir_all(&long));
                        match res {
                            Ok(()) => {
                                transferred.fetch_add(1, Ordering::SeqCst);
                                bytes.fetch_add(entry.size, Ordering::SeqCst);
                                let mut g = lock_ok(&tracker);
                                if let Some(p) = g.ingest(
                                    &RoboLine::Extra { size: entry.size, name: entry.name.clone() },
                                    Instant::now(),
                                ) {
                                    drop(g);
                                    sink.progress(&p);
                                }
                            }
                            Err(e) => {
                                if e.kind() != std::io::ErrorKind::NotFound {
                                    failed.fetch_add(1, Ordering::SeqCst);
                                    sink.error_line(&format!(
                                        "Delete failed {}: {}",
                                        entry.full_path, e
                                    ));
                                    log_event(&format!(
                                        "sync delete failed {}: {}",
                                        hash_path(&entry.full_path),
                                        e
                                    ));
                                } else {
                                    transferred.fetch_add(1, Ordering::SeqCst);
                                    bytes.fetch_add(entry.size, Ordering::SeqCst);
                                    let mut g = lock_ok(&tracker);
                                    if let Some(p) = g.ingest(
                                        &RoboLine::Extra {
                                            size: entry.size,
                                            name: entry.name.clone(),
                                        },
                                        Instant::now(),
                                    ) {
                                        drop(g);
                                        sink.progress(&p);
                                    }
                                }
                            }
                        }
                    }
                    live_workers.fetch_sub(1, Ordering::SeqCst);
                });
            }
        });
        sync_delete_transferred = transferred.load(Ordering::SeqCst);
        sync_delete_failed = failed.load(Ordering::SeqCst);
        let sync_delete_bytes = bytes.load(Ordering::SeqCst);
        // Cleanup empty extra dirs after files are gone (sequential, bottom-up)
        let dirs_removed = cleanup_extra_empty_dirs(source, effective_dest, filter.as_deref());
        if dirs_removed > 0 {
            log_event(&format!("sync phase 1: removed {} empty extra dirs", dirs_removed));
        }
        if control.is_cancelled() {
            let s = WarpSummary {
                total_files: sync_delete_transferred + sync_delete_failed,
                transferred: sync_delete_transferred,
                skipped: 0,
                failed: sync_delete_failed,
                duration_ms: start.elapsed().as_millis() as u64,
                bytes_transferred: lock_ok(&tracker).bytes_done,
                cancelled: true,
                error_code: 0,
                error_message: String::new(),
                verified: false,
                verify_mismatches: 0,
                workers_used: workers as u32,
                retried_ok: 0,
            };
            log_event(&format!(
                "sync parallel cancelled during delete phase after {} ms",
                s.duration_ms
            ));
            return Ok(s);
        }
        if sync_delete_failed > 0 {
            // Strict gate: if any delete failed, abort before copy touches dest
            let s = WarpSummary {
                total_files: total_files_scan,
                transferred: sync_delete_transferred,
                skipped: 0,
                failed: sync_delete_failed,
                duration_ms: start.elapsed().as_millis() as u64,
                bytes_transferred: sync_delete_bytes,
                cancelled: false,
                error_code: 8,
                error_message: format!("Sync delete phase failed: {} extra file(s) could not be deleted — check permissions or locked files.", sync_delete_failed),
                verified: false,
                verify_mismatches: 0,
                workers_used: workers as u32,
                retried_ok: 0,
            };
            log_event(&format!(
                "sync delete phase failed: {} files, aborting copy",
                sync_delete_failed
            ));
            return Ok(s);
        }
        log_event(&format!(
            "sync phase 1 complete: {} files deleted, {} bytes",
            sync_delete_transferred, sync_delete_bytes
        ));
        // Reset live_workers decay already done; done_shards remains 0 for copy phase
    }

    // — Phase 2: parallel copy (or sole phase for non-sync) ------------------
    // For sync, copy runs with /E only (deletes already done). For copy/move same as before.
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                // Pause gate: finish nothing new while paused. In-flight
                // shards run to completion (honest semantics surfaced in UI).
                while control.is_paused() && !control.is_cancelled() {
                    std::thread::sleep(std::time::Duration::from_millis(80));
                }
                if control.is_cancelled() {
                    break;
                }
                let shard = lock_ok(&queue).pop_front();
                let Some(shard) = shard else { break };
                // File-chunk shards (Plan B flat-dir) use direct copy, not robocopy
                let is_chunk = shard.chunk_files.is_some();
                log_json(
                    "info",
                    "shard_start",
                    serde_json::json!({ "id": shard.id, "src_hash": hash_path(&shard.src), "chunk": is_chunk }),
                );
                live_workers.fetch_add(1, Ordering::SeqCst);
                let outcome = if is_chunk {
                    run_file_chunk_shard(control, &shard, Some(&tracker), sink.clone(), skip_conflict)
                } else {
                    let args = pool::shard_args_with_filter(
                        &shard.src,
                        &shard.dst,
                        skip_conflict,
                        shard.root_only,
                        move_mode,
                        mt_per_worker,
                        filter.as_deref(),
                    );
                    run_shard(control, shard.id, &args, Some(&tracker), sink.clone())
                };
                live_workers.fetch_sub(1, Ordering::SeqCst);
                match outcome {
                    Ok(outcome) => {
                        done_shards.fetch_add(1, Ordering::SeqCst);
                        lock_ok(&outcomes).push(outcome);
                    }
                    Err(e) => log_event(&format!("shard {} spawn failed: {}", shard.id, e)),
                }
            });
        }
    });

    // — Cancelled? Report partials and stop. --------------------------------
    if control.is_cancelled() {
        let mut s = WarpSummary {
            total_files: 0,
            transferred: 0,
            skipped: 0,
            failed: 0,
            duration_ms: start.elapsed().as_millis() as u64,
            bytes_transferred: lock_ok(&tracker).bytes_done,
            cancelled: true,
            error_code: 0,
            error_message: String::new(),
            verified: false,
            verify_mismatches: 0,
            workers_used: workers as u32,
            retried_ok: 0,
        };
        for o in lock_ok(&outcomes).iter() {
            s.total_files += o.transferred + o.skipped + o.failed;
            s.transferred += o.transferred;
            s.skipped += o.skipped;
            s.failed += o.failed;
        }
        // Include sync delete phase counts if applicable
        if is_sync {
            s.total_files += sync_delete_transferred + sync_delete_failed;
            s.transferred += sync_delete_transferred;
            s.failed += sync_delete_failed;
        }
        log_event(&format!(
            "parallel cancelled after {} ms, {}/{} files",
            s.duration_ms, s.transferred, s.total_files
        ));
        return Ok(s);
    }

    // — Retry pass — sequential, max 2 attempts per failed shard ------------
    // robocopy skips already-copied files ("Same"), so re-running a failed
    // shard only copies what is missing or was locked. Deliberately single-
    // threaded: failures cluster around file locks/network hiccups, and
    // hammering them in parallel makes them worse.
    const MAX_RETRY_ATTEMPTS: u32 = 2;
    let mut retried_ok = 0u32;
    let mut retry_ids: Vec<u64> = lock_ok(&outcomes)
        .iter()
        .filter(|o| o.exit_code >= 8 || o.failed > 0)
        .map(|o| o.id)
        .collect();
    retry_ids.sort_unstable();

    for id in retry_ids {
        for attempt in 0..MAX_RETRY_ATTEMPTS {
            if control.is_cancelled() {
                break;
            }
            let Some(prev) = lock_ok(&outcomes).iter().find(|o| o.id == id).cloned() else { break };
            if prev.failed == 0 && prev.exit_code < 8 {
                break;
            }
            let Some((src, dst, root_only, chunk_files)) = shard_meta.get(&id).cloned() else {
                break;
            };
            log_event(&format!(
                "retry shard {} attempt {}/{} ({} failed files)",
                id,
                attempt + 1,
                MAX_RETRY_ATTEMPTS,
                prev.failed
            ));
            // Roll back the failed attempt's byte contribution so the tracker
            // reflects exactly one completed pass over this shard afterwards.
            lock_ok(&tracker).revert_bytes(prev.counted_bytes);
            live_workers.fetch_add(1, Ordering::SeqCst);
            let retried = if let Some(files) = chunk_files {
                let shard = shards::Shard {
                    id,
                    src: src.clone(),
                    dst: dst.clone(),
                    est_bytes: 0,
                    est_files: files.len() as u64,
                    root_only,
                    chunk_files: Some(files),
                };
                run_file_chunk_shard(control, &shard, Some(&tracker), sink.clone(), skip_conflict)
            } else {
                let args = pool::shard_args_with_filter(
                    &src,
                    &dst,
                    skip_conflict,
                    root_only,
                    move_mode,
                    mt_per_worker,
                    filter.as_deref(),
                );
                run_shard(control, id, &args, Some(&tracker), sink.clone())
            };
            live_workers.fetch_sub(1, Ordering::SeqCst);
            match retried {
                Ok(fresh) => {
                    if fresh.failed == 0 {
                        done_shards.fetch_add(1, Ordering::SeqCst);
                    }
                    retried_ok += pool::recovered_from_retry(prev.failed, fresh.failed);
                    let mut outs = lock_ok(&outcomes);
                    if let Some(slot) = outs.iter_mut().find(|o| o.id == id) {
                        *slot = fresh;
                    } else {
                        outs.push(fresh);
                    }
                }
                Err(e) => {
                    log_event(&format!("retry shard {} spawn failed: {}", id, e));
                    break;
                }
            }
        }
    }
    if control.is_cancelled() {
        // Cancel arrived during retries — report partials like above.
        let mut s = WarpSummary {
            total_files: 0,
            transferred: 0,
            skipped: 0,
            failed: 0,
            duration_ms: start.elapsed().as_millis() as u64,
            bytes_transferred: lock_ok(&tracker).bytes_done,
            cancelled: true,
            error_code: 0,
            error_message: String::new(),
            verified: false,
            verify_mismatches: 0,
            workers_used: workers as u32,
            retried_ok,
        };
        for o in lock_ok(&outcomes).iter() {
            s.total_files += o.transferred + o.skipped + o.failed;
            s.transferred += o.transferred;
            s.skipped += o.skipped;
            s.failed += o.failed;
        }
        if is_sync {
            s.total_files += sync_delete_transferred + sync_delete_failed;
            s.transferred += sync_delete_transferred;
            s.failed += sync_delete_failed;
        }
        return Ok(s);
    }

    // — Assemble summary from final per-shard outcomes -----------------------
    let mut s = WarpSummary {
        total_files: 0,
        transferred: 0,
        skipped: 0,
        failed: 0,
        duration_ms: 0,
        bytes_transferred: 0,
        cancelled: false,
        error_code: 0,
        error_message: String::new(),
        verified: false,
        verify_mismatches: 0,
        workers_used: workers as u32,
        retried_ok,
    };
    {
        let outs = lock_ok(&outcomes);
        for o in outs.iter() {
            s.total_files += o.transferred + o.skipped + o.failed;
            s.transferred += o.transferred;
            s.skipped += o.skipped;
            s.failed += o.failed;
        }
        let codes: Vec<i32> =
            outs.iter().filter(|o| o.had_exit_code).map(|o| o.exit_code).collect();
        let worst = codes.iter().copied().max().unwrap_or(0);
        s.error_code = worst;
        if worst >= 8 {
            s.error_message = robocopy_exit_message(worst)
                .unwrap_or_else(|| format!("Transfer failed (exit code {})", worst));
        } else if outs.iter().any(|o| !o.had_exit_code) {
            s.error_message =
                "Transfer terminated unexpectedly (no exit code — process was killed)".to_string();
            s.error_code = -1;
        }
    }
    if is_sync {
        s.total_files += sync_delete_transferred + sync_delete_failed;
        s.transferred += sync_delete_transferred;
        s.failed += sync_delete_failed;
        // bytes_transferred already includes delete bytes via tracker, so just keep it
        if sync_delete_failed > 0 && s.error_code < 8 {
            s.error_code = 8;
            s.error_message = format!("Sync delete phase had {} failure(s)", sync_delete_failed);
        }
    }
    {
        let t = lock_ok(&tracker);
        s.bytes_transferred = t.bytes_done;
    }
    s.duration_ms = start.elapsed().as_millis() as u64;

    // — Post-transfer tail — mirrors the sequential engine -------------------
    if s.error_code >= 8 || s.error_code < 0 && !s.error_message.is_empty() && s.error_code == -1 {
        log_event(&format!(
            "parallel failed code={} transferred={} skipped={} failed={} msg={}",
            s.error_code, s.transferred, s.skipped, s.failed, s.error_message
        ));
        return Ok(s); // same contract as sequential: Ok(summary) carrying the error
    }

    log_event(&format!(
        "parallel success code={} transferred={} skipped={} failed={} bytes={} retried_ok={}",
        s.error_code, s.transferred, s.skipped, s.failed, s.bytes_transferred, s.retried_ok
    ));

    if move_mode && s.failed == 0 {
        trash::log_trash(mode, source, effective_dest, vec![]);
        remove_empty_dirs(std::path::Path::new(source));
    }
    if mode == "sync" && s.failed == 0 {
        trash::log_trash(mode, source, effective_dest, vec![]);
    }

    if verify && mode != "move" && s.failed == 0 {
        let _ = window.emit("warp-verifying", ());
        s.verify_mismatches = verify_transfer(source, effective_dest);
        s.verified = true;
        log_event(&format!("verify mismatches={}", s.verify_mismatches));
    }

    Ok(s)
}

// — Entry ---------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(TransferControl::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;
            trash::prune_old();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            warp_file_op,
            get_path_info,
            cancel_warp,
            pause_warp,
            undo_last,
            check_health
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| match event {
            // Safety net: whatever closes the window or quits the app (custom
            // close button, Alt+F4, taskbar, updater restart), kill robocopy
            // first so it can never outlive Warp.
            tauri::RunEvent::WindowEvent { event: tauri::WindowEvent::Destroyed, .. } => {
                kill_active_children(app)
            }
            tauri::RunEvent::Exit => kill_active_children(app),
            _ => {}
        });
}

// — Tests ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preflight::max_file_size;
    use crate::progress::{fmt_bytes_pretty, ipg_for_throttle};

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
        assert_eq!(ipg_for_throttle(5), Some(13)); // 62.5/5 = 12.5 -> 13
        assert_eq!(ipg_for_throttle(25), Some(3)); // 62.5/25 = 2.5 -> 3 (rounds half away from zero)
        assert_eq!(ipg_for_throttle(100), Some(1)); // 62.5/100 = 0.625 -> 1
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
        // Dest-only files are prefixed with '*' (locale-independent).
        // For Copy/Move they still parse as Extra but scan() only counts them
        // for Sync (/MIR) so totals stay correct; verify still ignores them.
        let line = "\t  *EXTRA File \t\t      12\tz.txt";
        match parse_line(line) {
            RoboLine::Extra { size, name } => {
                assert_eq!(size, 12);
                assert_eq!(name, "z.txt");
            }
            _ => panic!("expected Extra for *EXTRA line"),
        }
    }

    #[test]
    fn parse_timestamped_error_line() {
        // Real error log lines carry a timestamp and a locale-independent
        // "<code> (0x<hex>)" pair. Sharing violation (32) now includes hint.
        let line =
            "2026/08/06 21:12:33 ERROR 32 (0x00000020) Copying File C:\\tmp\\src\\locked.txt";
        match parse_line(line) {
            RoboLine::FileHeader { is_error, name, .. } => {
                assert!(is_error);
                assert!(name.contains("locked.txt"));
                assert!(name.contains("file in use"));
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

    #[test]
    fn to_long_path_handles_normal_and_long() {
        assert_eq!(to_long_path(r"C:\short"), r"C:\short");
        assert_eq!(to_long_path(r"\\server\share\file"), r"\\?\UNC\server\share\file");
        let long = format!("C:\\{}", "a".repeat(300));
        assert!(to_long_path(&long).starts_with(r"\\?\"));
        assert!(to_long_path(r"\\?\C:\already").starts_with(r"\\?\"));
    }

    #[test]
    fn max_file_size_sparse() {
        let base = std::env::temp_dir().join(format!("warp_max_{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let p = base.join("sparse.dat");
        let f = std::fs::File::create(&p).unwrap();
        f.set_len(100 * 1024 * 1024).unwrap();
        drop(f);
        assert_eq!(max_file_size(&base.to_string_lossy()), 100 * 1024 * 1024);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn parse_percent_line() {
        match parse_line("  12.3%") {
            RoboLine::Percent(p) => assert!((p - 12.3).abs() < 0.01),
            _ => panic!("expected Percent"),
        }
        match parse_line("  100%") {
            RoboLine::Percent(p) => assert_eq!(p, 100.0),
            _ => panic!("expected Percent 100"),
        }
        match parse_line("  0%  New File  1.0g  C:\\big.dat") {
            RoboLine::Percent(p) => assert_eq!(p, 0.0),
            _ => panic!("expected Percent with file"),
        }
    }

    #[test]
    fn fmt_bytes_pretty_scales() {
        assert_eq!(fmt_bytes_pretty(512), "512 B");
        assert_eq!(fmt_bytes_pretty(2048), "2 KB");
        assert_eq!(fmt_bytes_pretty(5 * 1_048_576), "5.0 MB");
        assert_eq!(fmt_bytes_pretty(3 * 1_073_741_824), "3.0 GB");
    }
}

// — Updater signature test ---------------------------------------------------
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
        let bundle = manifest_dir.join("target").join("release").join("bundle").join("nsis");
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
            .rfind(|l| !l.is_empty())
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
        let signature =
            minisign_verify::Signature::decode(&sig_text).expect("failed to parse the .sig file");
        let data = std::fs::read(&exe).unwrap();

        pk.verify(&data, &signature, false)
            .expect("installer signature must verify against the configured pubkey");
    }
}

// — Real-robocopy integration tests ------------------------------------------
//
// These shell out to the REAL robocopy binary (the same commands the app
// builds) against real folders on disk, so the parser, scan pass, verify pass
// (with its exit-code fallback), and move cleanup are validated against actual
// robocopy output rather than hand-written fixtures. Windows-only: robocopy
// ships with every Windows install but does not exist elsewhere.
#[cfg(all(test, windows))]
mod real_robocopy {
    use super::*;
    use crate::engine_seq::scan;
    use crate::preflight::{free_bytes_available, is_removable_drive};
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
            src.to_string(),
            dst.to_string(),
            "/E".into(),
            "/NP".into(),
            "/R:3".into(),
            "/W:5".into(),
            "/BYTES".into(),
            "/NJH".into(),
            "/NJS".into(),
            "/256".into(),
            "/MT:32".into(),
        ]
    }

    #[test]
    fn scan_counts_the_real_tree() {
        let (base, src, dst) = setup("scan");
        let s = src.to_string_lossy().to_string();
        let d = dst.to_string_lossy().to_string();

        let (bytes, files) = scan(&s, &d, "copy");
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
        let out = robocopy_cmd().args(copy_args(&s, &d)).output().expect("robocopy must run");
        let code = out.status.code().unwrap();
        assert_eq!(code, 1, "exit 1 = files copied successfully");
        assert!(dst.join("a.txt").exists());
        assert!(dst.join("sub").join("c.txt").exists());

        // Identical trees -> verify reports zero mismatches (exit code 0 + parser).
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
                &s, &d, "/MOVE", "/E", "/NP", "/R:3", "/W:5", "/BYTES", "/NJH", "/NJS", "/256",
                "/MT:32",
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

    /// Full parallel-engine integration: partition a real tree, run two shards
    /// CONCURRENTLY through `run_shard` with a live TransferControl, then
    /// verify the merged destination exactly matches what one sequential copy
    /// would have produced (file set + zero verify mismatches).
    #[test]
    fn parallel_shards_copy_concurrently_and_verify_clean() {
        struct DevNullSink;
        impl ShardSink for DevNullSink {
            fn progress(&self, _p: &WarpProgress) {}
            fn error_line(&self, _s: &str) {}
        }

        let (base, src, dst) = setup("par");
        let s = src.to_string_lossy().to_string();
        let d = dst.to_string_lossy().to_string();

        let shards = shards::partition(&s, &d);
        assert!(shards.len() >= 2, "fixture must partition into >=2 shards");

        let control = TransferControl::default();
        let tracker = Mutex::new(pool::Tracker::new(10_000, 10, false, false));
        let sink: std::sync::Arc<dyn ShardSink> = std::sync::Arc::new(DevNullSink);

        std::thread::scope(|scope| {
            for shard in shards.iter() {
                let control = &control;
                let tracker = &tracker;
                let sink = sink.clone();
                scope.spawn(move || {
                    let args =
                        pool::shard_args(&shard.src, &shard.dst, false, shard.root_only, false, 4);
                    let outcome = run_shard(control, shard.id, &args, Some(tracker), sink)
                        .expect("shard robocopy must spawn");
                    assert!(outcome.exit_code < 8, "shard failed: {:?}", outcome);
                });
            }
        });

        // Every file landed, none lost between shards.
        assert!(dst.join("a.txt").exists());
        assert!(dst.join("b.txt").exists());
        assert!(dst.join("sub").join("c.txt").exists());
        assert_eq!(verify_transfer(&s, &d), 0, "merged tree must verify clean");

        // Tracker saw every file across both concurrent children.
        {
            let t = lock_ok(&tracker);
            assert_eq!(t.transferred, 3);
            // "hello world" + "second file" + "nested file" = 33 bytes.
            assert_eq!(t.bytes_done, 33);
        }

        // Cancel machinery reaches pool children: register is drained cleanly.
        assert!(lock_children(&control.children).is_empty());

        let _ = std::fs::remove_dir_all(&base);
    }

    // — Perf smoke harness (run explicitly: cargo test --lib — --ignored --nocapture perf) --

    struct PerfSink;
    impl ShardSink for PerfSink {
        fn progress(&self, _p: &WarpProgress) {}
        fn error_line(&self, _s: &str) {}
    }

    /// First removable drive as "X:" (None when no USB stick is attached).
    fn find_removable_drive() -> Option<String> {
        for b in b'D'..=b'Z' {
            let drive = format!("{}:", b as char);
            let root = format!(r"{}\", drive);
            if std::path::Path::new(&root).is_dir() && is_removable_drive(&drive) {
                return Some(drive);
            }
        }
        None
    }

    #[allow(dead_code)]
    struct PerfFixture {
        root: PathBuf,
        dirs: usize,
        files_per_dir: usize,
        file_bytes: usize,
    }

    impl Drop for PerfFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    /// Generates `dirs` x `files_per_dir` small files — the many-small-files
    /// shape where directory enumeration dominates and parallel shards win.
    fn make_perf_fixture(
        tag: &str,
        base: &std::path::Path,
        dirs: usize,
        files_per_dir: usize,
        file_bytes: usize,
    ) -> PerfFixture {
        let root = base.join(format!("warp_perf_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let payload = vec![0xA5u8; file_bytes];
        for d in 0..dirs {
            let dir = root.join(format!("d{d:03}"));
            std::fs::create_dir_all(&dir).unwrap();
            for f in 0..files_per_dir {
                std::fs::write(dir.join(format!("f{f:04}.bin")), &payload).unwrap();
            }
        }
        PerfFixture { root, dirs, files_per_dir, file_bytes }
    }

    /// The sequential engine's exact fast-path argument set (/MT:32).
    fn run_sequential_copy(src: &str, dst: &str) -> u128 {
        let t0 = Instant::now();
        let out = robocopy_cmd()
            .args([
                src, dst, "/E", "/NP", "/R:3", "/W:5", "/BYTES", "/NJH", "/NJS", "/256", "/MT:32",
            ])
            .output()
            .expect("robocopy must run");
        assert!(out.status.code().unwrap() < 8, "sequential copy failed");
        t0.elapsed().as_millis()
    }

    /// The coordinator's real path: partition -> bounded worker pool -> run_shard
    /// with per-worker /MT:8 — exactly what `transfer_parallel` does.
    fn run_parallel_copy(src: &str, dst: &str, workers_req: usize) -> (u128, usize) {
        let shard_list = shards::partition(src, dst);
        assert!(shard_list.len() >= 2, "fixture must partition into >= 2 shards");
        let workers = workers_req.min(shard_list.len()).max(1);
        let control = TransferControl::default();
        let tracker = Mutex::new(pool::Tracker::new(0, 0, false, false));
        let sink: std::sync::Arc<dyn ShardSink> = std::sync::Arc::new(PerfSink);
        let queue: Mutex<VecDeque<shards::Shard>> = Mutex::new(shard_list.into_iter().collect());
        let t0 = Instant::now();
        std::thread::scope(|scope| {
            for _ in 0..workers {
                scope.spawn(|| loop {
                    let shard = lock_ok(&queue).pop_front();
                    let Some(shard) = shard else { break };
                    let args =
                        pool::shard_args(&shard.src, &shard.dst, false, shard.root_only, false, 8);
                    let outcome =
                        run_shard(&control, shard.id, &args, Some(&tracker), sink.clone())
                            .expect("shard robocopy must spawn");
                    assert!(outcome.exit_code < 8, "parallel shard failed: {outcome:?}");
                });
            }
        });
        (t0.elapsed().as_millis(), workers)
    }

    fn count_files(root: &std::path::Path) -> u64 {
        let (bytes, files) = dir_stats(&root.to_string_lossy());
        let _ = bytes;
        files
    }

    fn report(tag: &str, total_mb: f64, seq_ms: u128, par_ms: u128, workers: usize) {
        let seq_mbps = total_mb / (seq_ms.max(1) as f64 / 1000.0);
        let par_mbps = total_mb / (par_ms.max(1) as f64 / 1000.0);
        println!("[perf:{tag}] sequential(/MT:32): {seq_ms} ms ({seq_mbps:.1} MB/s)");
        println!("[perf:{tag}] parallel(W={workers}):      {par_ms} ms ({par_mbps:.1} MB/s)");
        println!(
            "[perf:{tag}] speedup: {:.2}x {}",
            seq_ms as f64 / par_ms.max(1) as f64,
            if par_ms < seq_ms { "(faster)" } else { "(NOT faster — contention or noise)" }
        );
    }

    /// NVMe smoke: many-small-files tree, sequential vs 6-worker pool.
    /// Both destinations are verified against the source afterwards.
    #[test]
    #[ignore = "perf smoke: cargo test --lib — --ignored --nocapture perf_local"]
    fn perf_local() {
        let (dirs, fpd, fb) = (30, 300, 16 * 1024); // 9000 files ~ 144 MB
        let tmp = std::env::temp_dir();
        let fx = make_perf_fixture("local", &tmp, dirs, fpd, fb);
        let src = fx.root.to_string_lossy().to_string();
        let total_mb = (dirs * fpd * fb) as f64 / (1024.0 * 1024.0);
        let src_files = count_files(fx.root.as_path());

        // Warm metadata cache once so both runs see the same conditions.
        let _ = scan(&src, &src, "copy");

        let dst_seq_p = tmp.join(format!("warp_perf_dst_seq_{}", std::process::id()));
        let dst_par_p = tmp.join(format!("warp_perf_dst_par_{}", std::process::id()));

        let seq_ms = run_sequential_copy(&src, &dst_seq_p.to_string_lossy());
        let (par_ms, w) = run_parallel_copy(&src, &dst_par_p.to_string_lossy(), 6);

        // Integrity: both engines produced complete, verifiable trees.
        let dst_seq = dst_seq_p.to_string_lossy().to_string();
        let dst_par = dst_par_p.to_string_lossy().to_string();
        assert_eq!(count_files(&dst_seq_p), src_files, "sequential tree incomplete");
        assert_eq!(count_files(&dst_par_p), src_files, "parallel tree incomplete");
        assert_eq!(verify_transfer(&src, &dst_seq), 0, "sequential verify");
        assert_eq!(verify_transfer(&src, &dst_par), 0, "parallel verify");

        report("local-nvme", total_mb, seq_ms, par_ms, w);
        println!("[perf:local-nvme] fixture: {dirs} dirs x {fpd} files x {fb}B = {total_mb:.0} MB, {src_files} files");

        let _ = std::fs::remove_dir_all(&dst_seq_p);
        let _ = std::fs::remove_dir_all(&dst_par_p);
    }

    /// USB smoke: same comparison against a removable stick with the auto-policy
    /// worker cap (2). Skips cleanly when no removable drive is present — plug
    #[test]
    fn sync_collect_extra_files_and_cleanup() {
        let base = std::env::temp_dir().join(format!("warp_sync_{}", std::process::id()));
        let src = base.join("src");
        let dst = base.join("dst");
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::create_dir_all(dst.join("sub")).unwrap();
        std::fs::create_dir_all(dst.join("extra_dir")).unwrap();
        std::fs::write(src.join("a.txt"), b"hello").unwrap();
        std::fs::write(src.join("sub/b.txt"), b"world").unwrap();
        std::fs::write(dst.join("a.txt"), b"hello").unwrap();
        std::fs::write(dst.join("extra.txt"), b"junk").unwrap();
        std::fs::write(dst.join("sub/b.txt"), b"world").unwrap();
        std::fs::write(dst.join("extra_dir/c.txt"), b"extra").unwrap();
        // extra files should be extra.txt and extra_dir/c.txt (2)
        let extras = collect_extra_files(&src.to_string_lossy(), &dst.to_string_lossy(), None);
        assert_eq!(
            extras.len(),
            2,
            "extras={:?}",
            extras.iter().map(|e| &e.full_path).collect::<Vec<_>>()
        );
        assert!(extras.iter().any(|e| e.name == "extra.txt"));
        assert!(extras.iter().any(|e| e.name == "c.txt"));
        // Filter should exclude *.txt from extras if we set filter
        let filtered =
            collect_extra_files(&src.to_string_lossy(), &dst.to_string_lossy(), Some("*.txt"));
        assert_eq!(filtered.len(), 0, "filtered extras should be 0");
        // Cleanup empty extra dirs after removing files
        for e in &extras {
            let _ = std::fs::remove_file(e.full_path.clone());
        }
        let removed =
            cleanup_extra_empty_dirs(&src.to_string_lossy(), &dst.to_string_lossy(), None);
        assert_eq!(removed, 1, "extra_dir should be removed");
        assert!(!dst.join("extra_dir").exists());
        assert!(dst.join("sub").exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    /// one in and re-run: cargo test --lib — --ignored --nocapture perf_usb
    #[test]
    #[ignore = "perf smoke (needs USB stick): cargo test --lib — --ignored --nocapture perf_usb"]
    fn perf_usb() {
        let Some(drive) = find_removable_drive() else {
            println!(
                "[perf:usb] SKIP — no removable drive detected. Plug in a USB stick and re-run."
            );
            return;
        };
        let free = free_bytes_available(&format!(r"{drive}\")).unwrap_or(0);
        assert!(free > 512 * 1024 * 1024, "USB drive {drive} needs >512 MB free");

        let (dirs, fpd, fb) = (12, 150, 16 * 1024); // 1800 files ~ 29 MB — kind to flash wear
        let base = std::path::PathBuf::from(format!(r"{drive}\"));
        let fx = make_perf_fixture("usb", &base, dirs, fpd, fb);
        let src = fx.root.to_string_lossy().to_string();
        let total_mb = (dirs * fpd * fb) as f64 / (1024.0 * 1024.0);
        let src_files = count_files(fx.root.as_path());

        let policy_w = pool::resolve_workers_for(
            0,
            "copy",
            0,
            usize::MAX,
            src_files,
            (dirs * fpd * fb) as u64,
            true,
            false,
        );
        assert_eq!(policy_w, 2, "auto policy must cap USB at 2 workers");
        println!("[perf:usb] drive {drive} — auto policy selects W={policy_w}");

        let dst_seq_p = base.join(format!("warp_perf_dst_seq_{}", std::process::id()));
        let dst_par_p = base.join(format!("warp_perf_dst_par_{}", std::process::id()));

        let seq_ms = run_sequential_copy(&src, &dst_seq_p.to_string_lossy());
        let (par_ms, w) = run_parallel_copy(&src, &dst_par_p.to_string_lossy(), policy_w);
        assert_eq!(w, policy_w);

        let dst_seq = dst_seq_p.to_string_lossy().to_string();
        let dst_par = dst_par_p.to_string_lossy().to_string();
        assert_eq!(count_files(&dst_seq_p), src_files, "sequential tree incomplete");
        assert_eq!(count_files(&dst_par_p), src_files, "parallel tree incomplete");
        assert_eq!(verify_transfer(&src, &dst_seq), 0, "sequential verify");
        assert_eq!(verify_transfer(&src, &dst_par), 0, "parallel verify");

        report("usb", total_mb, seq_ms, par_ms, w);

        let _ = std::fs::remove_dir_all(&dst_seq_p);
        let _ = std::fs::remove_dir_all(&dst_par_p);
    }
}
