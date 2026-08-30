#![allow(dead_code)]
// Trash log for Move/Sync undo — 7-day JSON in %TEMP%\.warp-trash

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct TrashEntry {
    pub ts: u64,
    pub mode: String,
    pub src: String,
    pub dst: String,
    pub deleted: Vec<String>,
}

fn trash_dir() -> PathBuf {
    std::env::temp_dir().join(".warp-trash")
}

pub fn prune_old() {
    let dir = trash_dir();
    let Ok(rd) = std::fs::read_dir(&dir) else { return };
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    for entry in rd.flatten() {
        let path = entry.path();
        if let Ok(meta) = entry.metadata() {
            if let Ok(modified) = meta.modified() {
                if let Ok(age) = modified.duration_since(UNIX_EPOCH) {
                    let secs = now.saturating_sub(age.as_secs());
                    if secs > 7 * 24 * 3600 {
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
        // Also prune by filename timestamp if metadata fails
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if let Some(ts_str) =
                name.strip_prefix("warp-trash-").and_then(|s| s.strip_suffix(".json"))
            {
                if let Ok(ts) = ts_str.parse::<u64>() {
                    if now.saturating_sub(ts) > 7 * 24 * 3600 {
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
    }
}

pub fn log_trash(mode: &str, src: &str, dst: &str, deleted: Vec<String>) {
    if deleted.is_empty() && mode != "move" && mode != "sync" {
        return;
    }
    let dir = trash_dir();
    let _ = std::fs::create_dir_all(&dir);
    prune_old();
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let entry = TrashEntry {
        ts,
        mode: mode.to_string(),
        src: src.to_string(),
        dst: dst.to_string(),
        deleted,
    };
    let path = dir.join(format!("warp-trash-{}.json", ts));
    if let Ok(json) = serde_json::to_string_pretty(&entry) {
        let _ = std::fs::write(&path, json);
    }
    // Cap at 50 MB / 1000 entries: if dir >50MB, remove oldest
    if let Ok(rd) = std::fs::read_dir(&dir) {
        let mut files: Vec<_> = rd
            .flatten()
            .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
            .collect();
        if files.len() > 1000 {
            files.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
            for f in files.iter().take(files.len() - 1000) {
                let _ = std::fs::remove_file(f.path());
            }
        }
        let total: u64 = files.iter().filter_map(|e| e.metadata().ok().map(|m| m.len())).sum();
        if total > 50 * 1024 * 1024 {
            files.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
            let mut to_remove = total - 50 * 1024 * 1024;
            for f in files {
                if to_remove == 0 {
                    break;
                }
                if let Ok(m) = f.metadata() {
                    let _ = std::fs::remove_file(f.path());
                    to_remove = to_remove.saturating_sub(m.len());
                }
            }
        }
    }
}

pub fn latest_trash() -> Option<(PathBuf, TrashEntry)> {
    let dir = trash_dir();
    let rd = std::fs::read_dir(&dir).ok()?;
    let mut latest: Option<(PathBuf, TrashEntry, u64)> = None;
    for entry in rd.flatten() {
        let path = entry.path();
        if path.extension().map(|x| x != "json").unwrap_or(true) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else { continue };
        let Ok(entry) = serde_json::from_str::<TrashEntry>(&content) else { continue };
        let ts = entry.ts;
        if latest.as_ref().map(|(_, _, old_ts)| ts > *old_ts).unwrap_or(true) {
            latest = Some((path, entry, ts));
        }
    }
    latest.map(|(p, e, _)| (p, e))
}

pub fn undo_last() -> Result<String, String> {
    let Some((path, entry)) = latest_trash() else {
        return Err("No undo history — no Move/Sync in the last 7 days".into());
    };
    // For Move: reverse is robocopy dst -> src /E /MOVE
    // For Sync: we logged deleted list but didn't backup content — best effort is to re-copy from dst's current state? Not recoverable.
    // For v2 we support Move undo only; Sync shows message.
    if entry.mode == "move" {
        let src = Path::new(&entry.src);
        let dst = Path::new(&entry.dst);
        // dst is effective dest (e.g., D:\Backup\Photos), src is C:\Photos
        // Reverse: move dst back to src's parent
        let src_parent = src.parent().ok_or("Invalid source")?;
        let dst_name = dst.file_name().ok_or("Invalid dest")?;
        let reverse_dest = src_parent.join(dst_name);
        // Use robocopy to move back: robocopy dst reverse_dest /E /MOVE
        let mut cmd = crate::robocopy_cmd();
        let out = cmd
            .args([
                dst.to_string_lossy().to_string(),
                reverse_dest.to_string_lossy().to_string(),
                "/E".into(),
                "/MOVE".into(),
                "/R:3".into(),
                "/W:5".into(),
            ])
            .output()
            .map_err(|e| e.to_string())?;
        let code = out.status.code().unwrap_or(-1);
        if code >= 8 {
            return Err(format!("Undo failed (robocopy exit {})", code));
        }
        let _ = std::fs::remove_file(&path);
        return Ok(format!("Undid Move: {} -> {}", entry.dst, entry.src));
    } else if entry.mode == "sync" {
        return Err("Undo for Sync not yet supported — deleted files were logged but not backed up. Check .warp-trash for list.".into());
    }
    Err("Unknown mode".into())
}
