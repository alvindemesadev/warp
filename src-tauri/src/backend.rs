#![allow(dead_code)]
// Backend trait — seam for future `rsync`/`rclone` without rewriting `lib.rs`.
// Phase 2: trait only, no new backend yet. `RobocopyBackend` wraps current `robocopy` impl.

use crate::pool::ShardOutcome;
use crate::shards::Shard;

/// Capabilities exposed by a backend (used for engine selection).
#[derive(Debug, Clone, Copy)]
pub struct Caps {
    pub supports_mirror: bool,
    pub supports_ipg: bool,
    pub max_path: usize,
}

/// Transfer backend — all filesystem work behind this trait.
/// `scan`/`copy_shard`/`verify` are the three verbs the coordinator needs.
pub trait TransferBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn caps(&self) -> Caps;
    /// List-only dry run -> (total_bytes, total_files).
    fn scan(&self, source: &str, dest: &str, mode: &str) -> (u64, u32);
    /// Copy one shard (disjoint source subtree) -> outcome.
    fn copy_shard(&self, shard: &Shard, opts: &CopyOpts) -> ShardOutcome;
    /// Structural verify (re-compare) -> mismatches.
    fn verify(&self, source: &str, dest: &str) -> u32;
}

#[derive(Debug, Clone)]
pub struct CopyOpts {
    pub mode: String,
    pub conflict: String,
    pub throttle: u32,
    pub verify: bool,
    pub workers: Option<u8>,
}

/// Current `robocopy` backend — thin wrapper around `lib.rs` helpers.
/// Implemented in `lib.rs` to avoid circular deps; this file defines the trait only.
pub struct RobocopyBackend;

impl TransferBackend for RobocopyBackend {
    fn name(&self) -> &'static str {
        "robocopy"
    }
    fn caps(&self) -> Caps {
        Caps { supports_mirror: true, supports_ipg: true, max_path: 32767 }
    }
    fn scan(&self, source: &str, dest: &str, mode: &str) -> (u64, u32) {
        crate::engine_seq::scan(source, dest, mode)
    }
    fn copy_shard(&self, shard: &Shard, _opts: &CopyOpts) -> ShardOutcome {
        ShardOutcome {
            id: shard.id,
            transferred: 0,
            skipped: 0,
            failed: 0,
            counted_bytes: 0,
            exit_code: 0,
            had_exit_code: true,
        }
    }
    fn verify(&self, source: &str, dest: &str) -> u32 {
        crate::verify::verify_transfer(source, dest)
    }
}

/// Native direct I/O backend (Phase 3.0) — direct Win32 kernel calls with Block Cloning on ReFS/DevDrive.
pub struct NativeBackend;

impl TransferBackend for NativeBackend {
    fn name(&self) -> &'static str {
        "native"
    }
    fn caps(&self) -> Caps {
        Caps { supports_mirror: false, supports_ipg: false, max_path: 32767 }
    }
    fn scan(&self, source: &str, _dest: &str, _mode: &str) -> (u64, u32) {
        let (bytes, files) = crate::dir_stats(source);
        (bytes, files as u32)
    }
    fn copy_shard(&self, shard: &Shard, opts: &CopyOpts) -> ShardOutcome {
        let is_cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let skip_existing = opts.conflict == "skip";
        let workers = opts.workers.unwrap_or(4) as usize;
        match crate::engine_native::run_native_transfer(
            &shard.src,
            &shard.dst,
            workers,
            skip_existing,
            is_cancel,
        ) {
            Ok(summary) => ShardOutcome {
                id: shard.id,
                transferred: summary.transferred,
                skipped: summary.skipped,
                failed: summary.failed,
                counted_bytes: summary.bytes_transferred,
                exit_code: if summary.failed > 0 {
                    8
                } else if summary.transferred > 0 {
                    1
                } else {
                    0
                },
                had_exit_code: true,
            },
            Err(_) => ShardOutcome {
                id: shard.id,
                transferred: 0,
                skipped: 0,
                failed: 1,
                counted_bytes: 0,
                exit_code: 16,
                had_exit_code: true,
            },
        }
    }
    fn verify(&self, source: &str, dest: &str) -> u32 {
        crate::verify::verify_transfer(source, dest)
    }
}

/// Placeholder for future `rsync` backend — compiles on non-Windows, not shipped.
#[cfg(not(windows))]
pub struct RsyncBackend;
#[cfg(not(windows))]
impl TransferBackend for RsyncBackend {
    fn name(&self) -> &'static str {
        "rsync"
    }
    fn caps(&self) -> Caps {
        Caps { supports_mirror: true, supports_ipg: false, max_path: 1024 }
    }
    fn scan(&self, _s: &str, _d: &str, _m: &str) -> (u64, u32) {
        unimplemented!("rsync backend — future")
    }
    fn copy_shard(&self, _s: &Shard, _o: &CopyOpts) -> ShardOutcome {
        unimplemented!("rsync backend — future")
    }
    fn verify(&self, _s: &str, _d: &str) -> u32 {
        unimplemented!("rsync backend — future")
    }
}
