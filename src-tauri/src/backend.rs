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
    fn scan(&self, _source: &str, _dest: &str, _mode: &str) -> (u64, u32) {
        // Real impl delegates to `crate::scan` — stubbed here to keep trait object safe.
        // `lib.rs` will provide the actual impl via inherent method until full split.
        (0, 0)
    }
    fn copy_shard(&self, _shard: &Shard, _opts: &CopyOpts) -> ShardOutcome {
        // Stub — real `pool::shard_args` + `run_shard` path lives in `lib.rs:pool`.
        ShardOutcome {
            id: 0,
            transferred: 0,
            skipped: 0,
            failed: 0,
            counted_bytes: 0,
            exit_code: 0,
            had_exit_code: true,
        }
    }
    fn verify(&self, _source: &str, _dest: &str) -> u32 {
        0
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
