use std::time::{Duration, Instant};

/// Convert to `\\?\` long-path form for Windows MAX_PATH bypass.
/// `C:\very\long` -> `\\?\C:\very\long`, `\\server\share` -> `\\?\UNC\server\share`.
/// Normalizes slashes and ignores already-prefixed paths.
pub fn to_long_path(p: &str) -> String {
    if p.starts_with(r"\\?\") {
        return p.to_string();
    }
    let normalized = p.replace('/', "\\");
    if let Some(stripped) = normalized.strip_prefix(r"\\") {
        return format!(r"\\?\UNC\{}", stripped);
    }
    if normalized.len() > 240 && std::path::Path::new(&normalized).is_absolute() {
        return format!(r"\\?\{}", normalized);
    }
    normalized
}

/// Extract the drive letter (e.g. "C:") from a path, or empty string.
pub fn extract_drive(path: &str) -> String {
    std::path::Path::new(path)
        .components()
        .next()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .unwrap_or_default()
}

#[cfg(windows)]
pub fn is_removable_drive(drive: &str) -> bool {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetDriveTypeW;
    let root = format!(r"{}\", drive.trim_end_matches(':'));
    let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
    const DRIVE_REMOVABLE: u32 = 2;
    unsafe { GetDriveTypeW(PCWSTR(wide.as_ptr())) == DRIVE_REMOVABLE }
}

#[cfg(not(windows))]
pub fn is_removable_drive(_drive: &str) -> bool {
    false
}

#[allow(dead_code)]
#[cfg(windows)]
pub fn is_cloud_placeholder(path: &str) -> bool {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetFileAttributesW;
    const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x00400000;
    const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x00040000;
    const FILE_ATTRIBUTE_OFFLINE: u32 = 0x00001000;
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let attrs = unsafe { GetFileAttributesW(PCWSTR(wide.as_ptr())) };
    if attrs == u32::MAX {
        return false;
    }
    (attrs
        & (FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS
            | FILE_ATTRIBUTE_RECALL_ON_OPEN
            | FILE_ATTRIBUTE_OFFLINE))
        != 0
}

#[cfg(not(windows))]
pub fn is_cloud_placeholder(_: &str) -> bool {
    false
}

#[cfg(windows)]
pub fn is_fat32_volume(path: &str) -> bool {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetVolumeInformationW;
    let drive = extract_drive(path);
    if drive.is_empty() {
        return false;
    }
    let root = format!(r"{}\", drive.trim_end_matches(':'));
    let wide_root: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
    let mut fs_buf = [0u16; 32];
    unsafe {
        let res = GetVolumeInformationW(
            PCWSTR(wide_root.as_ptr()),
            None,
            None,
            None,
            None,
            Some(&mut fs_buf),
        );
        if res.is_err() {
            return false;
        }
        let len = fs_buf.iter().position(|&c| c == 0).unwrap_or(fs_buf.len());
        let name = String::from_utf16_lossy(&fs_buf[..len]);
        name.eq_ignore_ascii_case("FAT32")
    }
}

#[cfg(not(windows))]
pub fn is_fat32_volume(_path: &str) -> bool {
    false
}

#[cfg(windows)]
pub fn free_bytes_available(path: &str) -> Option<u64> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let mut free: u64 = 0;
    unsafe {
        let res =
            GetDiskFreeSpaceExW(PCWSTR(wide.as_ptr()), Some(&mut free as *mut u64), None, None);
        if res.is_ok() {
            Some(free)
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
pub fn free_bytes_available(_path: &str) -> Option<u64> {
    None
}

pub fn health_mbps(dest: &str) -> Option<f64> {
    use std::io::Write;
    let probe = std::path::Path::new(dest).join(".warp-health-probe");
    let start = Instant::now();
    let data = vec![0u8; 64 * 1024];
    let res = (|| {
        let mut f =
            std::fs::OpenOptions::new().write(true).create(true).truncate(true).open(&probe)?;
        f.write_all(&data)?;
        f.sync_all()?;
        Ok::<_, std::io::Error>(())
    })();
    let elapsed = start.elapsed().as_secs_f64();
    let _ = std::fs::remove_file(&probe);
    if res.is_err() || elapsed <= 0.0 {
        return None;
    }
    Some((64.0 * 1024.0) / elapsed / 1024.0 / 1024.0)
}

/// Returns true if the given path is on a removable (USB) drive.
pub fn is_path_on_usb(path: &str) -> bool {
    let drive = extract_drive(path);
    if drive.is_empty() {
        return false;
    }
    if is_removable_drive(&drive) {
        return true;
    }
    matches!(drive.chars().next(), Some(c) if matches!(c, 'D'..='Z' | 'd'..='z'))
}

/// Returns largest file size in `dir` (or 0 if none). Iterative, skips symlinks, caps early at >4GB for FAT32 preflight.
pub fn max_file_size(dir: &str) -> u64 {
    let mut max = 0u64;
    let mut stack = vec![std::path::PathBuf::from(to_long_path(dir))];
    while let Some(current) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&current) {
            for entry in rd.flatten() {
                if let Ok(ft) = entry.file_type() {
                    if ft.is_symlink() {
                        continue;
                    }
                }
                if let Ok(m) = entry.metadata() {
                    if m.is_file() {
                        let sz = m.len();
                        if sz > max {
                            max = sz;
                            if max > 4_294_967_295 {
                                return max;
                            }
                        }
                    } else if m.is_dir() {
                        stack.push(entry.path());
                    }
                }
            }
        }
    }
    max
}

/// Destination path resolution.
pub fn resolve_effective_dest(source: &str, destination: &str, folder_mode: &str) -> String {
    let source_name =
        std::path::Path::new(source).file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();

    if source_name.is_empty() || folder_mode == "merge" {
        destination.to_string()
    } else {
        let dest_clean = destination.trim_end_matches(['\\', '/']);
        let dest_last =
            std::path::Path::new(dest_clean).file_name().and_then(|n| n.to_str()).unwrap_or("");

        if dest_last.eq_ignore_ascii_case(&source_name) {
            destination.to_string()
        } else {
            format!("{}\\{}", dest_clean, source_name)
        }
    }
}

/// Overlapping path guard (prevent copying a folder into itself).
pub fn check_overlap(source: &str, effective_dest: &str) -> Result<(), String> {
    fn norm(p: &str) -> String {
        p.replace('\\', "/").trim_end_matches('/').to_lowercase()
    }
    fn canonical_norm(p: &str) -> String {
        let long = to_long_path(p);
        if let Ok(canon) = std::fs::canonicalize(&long) {
            let s = canon.to_string_lossy().to_string();
            let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
            let s = if let Some(stripped) = s.strip_prefix(r"UNC\") {
                format!(r"\\{}", stripped)
            } else {
                s.to_string()
            };
            return s.replace('\\', "/").trim_end_matches('/').to_lowercase();
        }
        if let Some(parent) = std::path::Path::new(p).parent().and_then(|p| p.to_str()) {
            if !parent.is_empty() {
                if let Ok(canon_parent) = std::fs::canonicalize(to_long_path(parent)) {
                    let parent_str = canon_parent.to_string_lossy().to_string();
                    let parent_str = parent_str.strip_prefix(r"\\?\").unwrap_or(&parent_str);
                    let parent_str = if let Some(stripped) = parent_str.strip_prefix(r"UNC\") {
                        format!(r"\\{}", stripped)
                    } else {
                        parent_str.to_string()
                    };
                    if let Some(name) = std::path::Path::new(p).file_name().and_then(|n| n.to_str())
                    {
                        let joined = format!(
                            "{}/{}",
                            parent_str.replace('\\', "/").trim_end_matches('/'),
                            name
                        );
                        return joined.to_lowercase();
                    }
                }
            }
        }
        norm(p)
    }
    let a = canonical_norm(source);
    let b = canonical_norm(effective_dest);
    if !a.is_empty() && !b.is_empty() {
        if a == b {
            crate::log_event("blocked: same folder");
            return Err(
                "Source and destination are the same folder — choose a different destination."
                    .to_string(),
            );
        }
        if b.starts_with(&format!("{}/", a)) {
            crate::log_event("blocked: dest inside source");
            return Err(
                "Destination is inside the source — copying would recurse into itself.".to_string()
            );
        }
        if a.starts_with(&format!("{}/", b)) {
            crate::log_event("blocked: source inside dest");
            return Err(
                "Source is inside the destination — this may cause infinite recursion.".to_string()
            );
        }
    }
    Ok(())
}

/// Network share reachability preflight with timeout to prevent hanging on unreachable UNC paths.
pub fn check_network_dest(effective_dest: &str) -> Result<(), String> {
    if effective_dest.starts_with(r"\\") {
        let parts: Vec<&str> = effective_dest.split('\\').filter(|s| !s.is_empty()).collect();
        let target_path = if parts.len() >= 2 {
            format!(r"\\{}\{}", parts[0], parts[1])
        } else {
            effective_dest.to_string()
        };

        let check_path = target_path.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let res = std::fs::metadata(to_long_path(&check_path)).is_ok();
            let _ = tx.send(res);
        });

        // Bound reachability check to 3 seconds max
        match rx.recv_timeout(Duration::from_secs(3)) {
            Ok(true) => Ok(()),
            Ok(false) => {
                crate::log_event(&format!("blocked: network unreachable {}", target_path));
                Err(format!(
                    "Network path not reachable: {} — check connection, VPN, and credentials. The share may be offline.",
                    target_path
                ))
            }
            Err(_) => {
                crate::log_event(&format!("blocked: network timeout {}", target_path));
                Err(format!(
                    "Network path timed out after 3s: {} — server is unreachable or offline.",
                    target_path
                ))
            }
        }
    } else {
        Ok(())
    }
}

/// FAT32 per-file limit preflight (4 GiB - 1).
pub fn check_fat32_source(source: &str, effective_dest: &str) -> Result<(), String> {
    if is_fat32_volume(effective_dest) {
        let max = max_file_size(source);
        if max > 4_294_967_295 {
            crate::log_event(&format!("blocked: FAT32 limit max={}", max));
            return Err(format!(
                "Destination is FAT32 — cannot store files larger than 4 GB (found {}). Reformat the drive to NTFS or exFAT or choose another destination.",
                crate::progress::fmt_bytes_pretty(max)
            ));
        }
    }
    Ok(())
}

/// Free space preflight — requires `total_bytes` plus 100 MB headroom.
pub fn ensure_free_space(
    destination: &str,
    effective_dest: &str,
    total_bytes: u64,
) -> Result<(), String> {
    if total_bytes == 0 {
        return Ok(());
    }
    if let Some(free) = free_bytes_available(effective_dest)
        .or_else(|| free_bytes_available(destination))
        .or_else(|| {
            let d = extract_drive(effective_dest);
            if d.is_empty() {
                None
            } else {
                free_bytes_available(&format!(r"{}\", d.trim_end_matches(':')))
            }
        })
    {
        let need = total_bytes.saturating_add(100 * 1024 * 1024);
        if free < need {
            crate::log_event(&format!("blocked: no space need={} free={}", need, free));
            return Err(format!(
                "Not enough free space on destination: need {} but only {} available. Free up space or choose another drive.",
                crate::progress::fmt_bytes_pretty(need),
                crate::progress::fmt_bytes_pretty(free)
            ));
        }
    }
    Ok(())
}
