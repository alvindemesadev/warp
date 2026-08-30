#![allow(dead_code)]
// Progress helpers — single source for %/speed math. Extracted from lib.rs for Phase 2.
// Parity with TypeScript `src/lib/format.ts:10` — keep thresholds/rounding identical.

/// `0..99` clamp, never 100 mid-transfer.
pub fn overall_pct(done: u64, total: u64) -> u32 {
    if total == 0 {
        return 0;
    }
    ((done as f64 / total as f64) * 100.0).clamp(0.0, 99.0) as u32
}

/// Keep in sync with `src/lib/format.ts:10` — same thresholds, same `toFixed`.
pub fn fmt_speed(bps: u64) -> String {
    if bps >= 1_073_741_824 {
        format!("{:.1} GB/s", bps as f64 / 1_073_741_824.0)
    } else if bps >= 1_048_576 {
        format!("{:.0} MB/s", bps as f64 / 1_048_576.0)
    } else if bps >= 1_024 {
        format!("{:.0} KB/s", bps as f64 / 1_024.0)
    } else {
        format!("{} B/s", bps)
    }
}

/// Parity with `src/lib/format.ts:10` — `fmtBytes`.
pub fn fmt_bytes_pretty(b: u64) -> String {
    if b >= 1_073_741_824 {
        format!("{:.1} GB", b as f64 / 1_073_741_824.0)
    } else if b >= 1_048_576 {
        format!("{:.1} MB", b as f64 / 1_048_576.0)
    } else if b >= 1_024 {
        format!("{:.0} KB", b as f64 / 1_024.0)
    } else {
        format!("{} B", b)
    }
}

/// `N MB/s` -> `/IPG` ms. `None` = unlimited.
pub fn ipg_for_throttle(mb_per_sec: u32) -> Option<u64> {
    if mb_per_sec == 0 {
        None
    } else {
        Some(((62.5 / mb_per_sec as f64).round() as u64).max(1))
    }
}
