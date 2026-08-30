use crate::parser::{parse_line, RoboLine};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// Structural verification pass (existence + size + time).
///
/// Robocopy has no content-hash verification, so structural "verify" re-runs a
/// list-only (/L) comparison of source vs destination and counts how many files
/// robocopy would still copy. After a clean copy that count should be zero.
pub fn verify_transfer(source: &str, destination: &str) -> u32 {
    let out = match crate::robocopy_cmd()
        .args([source, destination, "/L", "/E", "/BYTES", "/NJH", "/NJS", "/NP"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return 0,
    };

    let mut mismatches = 0u32;
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if let RoboLine::FileHeader { is_same: false, is_error: false, .. } = parse_line(line) {
            mismatches += 1;
        }
    }

    let code = out.status.code();
    match code {
        Some(0) => 0,
        Some(_) => mismatches.max(1),
        None => mismatches.max(1),
    }
}

/// Compute SHA-256 hash of a file.
#[allow(dead_code)]
pub fn hash_file_sha256(path: &Path) -> Result<[u8; 32], std::io::Error> {
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    Ok(out)
}

/// Deep content verification: compares SHA-256 hashes of all matching files in source and destination.
#[allow(dead_code)]
pub fn verify_checksums_sha256(source: &str, destination: &str) -> Result<u32, String> {
    let src_path = Path::new(source);
    let dst_path = Path::new(destination);

    if !src_path.exists() || !dst_path.exists() {
        return Ok(1);
    }

    let mut mismatches = 0u32;
    let mut stack = vec![src_path.to_path_buf()];

    while let Some(current) = stack.pop() {
        let entries = match std::fs::read_dir(&current) {
            Ok(e) => e,
            Err(_) => {
                mismatches += 1;
                continue;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let relative = match path.strip_prefix(src_path) {
                Ok(r) => r,
                Err(_) => continue,
            };
            let dst_target = dst_path.join(relative);

            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                if !dst_target.is_file() {
                    mismatches += 1;
                    continue;
                }

                let src_hash = hash_file_sha256(&path).map_err(|e| e.to_string())?;
                let dst_hash = hash_file_sha256(&dst_target).map_err(|e| e.to_string())?;

                if src_hash != dst_hash {
                    mismatches += 1;
                }
            }
        }
    }

    Ok(mismatches)
}
