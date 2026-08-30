#![allow(clippy::too_many_arguments, clippy::lines_filter_map_ok)]
// Parallel transfer pool primitives — everything here is Tauri-free so it can
// be unit-tested in isolation.
//
//  - `Tracker`      : live display aggregation (bytes, %, speed EWMA, emit
//                     throttle). Faithful port of the sequential loop's math;
//                     the sequential engine keeps its own inline copy untouched
//                     so the shipped behavior can never silently drift.
//  - `shard_args`   : robocopy argument set for one shard.
//  - `resolve_workers_for` : contention-aware worker-count policy.
//  - `consume_stream` : parses one child's stdout into local shard counters +
//                     optional shared tracker events.

use std::io::BufRead;
use std::sync::Mutex;
use std::time::Instant;

use crate::{fmt_speed, overall_pct, parse_line, RoboLine, WarpProgress};

const LARGE_THRESHOLD: u64 = 10 * 1024 * 1024;
const EMIT_MIN_INTERVAL_MS: u128 = 150;
const SPEED_WINDOW_MS: u128 = 400;

// — Tracker -------------------------------------------------------------------

pub(crate) struct PendingLarge {
    size: u64,
    name: String,
    credited: u64,
}

/// Live aggregate state shared across workers. Counters here drive the real-
/// time UI only — the FINAL summary comes from per-shard `ShardOutcome`s.
pub(crate) struct Tracker {
    pub total_bytes: u64,
    pub files_total_scan: u32,
    pub indeterminate: bool,

    pub bytes_done: u64,
    pub transferred: u32,
    pub skipped: u32,
    pub failed: u32,
    pub files_seen: u32,

    /// Sequential mode defers >=10 MB files to per-file `%` lines for smooth
    /// single-file progress. Parallel mode disables deferral: several large
    /// files stream concurrently and a single pending-slot would misattribute.
    defer_large: bool,
    large_threshold: u64,
    pending_large: Option<PendingLarge>,

    last_emitted_pct: u32,
    last_emit_time: Instant,
    indeterminate_tick: u32,

    pub last_speed_str: String,
    last_bps: u64,
    speed_window_bytes: u64,
    speed_window_start: Instant,
}

impl Tracker {
    pub(crate) fn new(
        total_bytes: u64,
        files_total_scan: u32,
        indeterminate: bool,
        defer_large: bool,
    ) -> Self {
        Self {
            total_bytes,
            files_total_scan,
            indeterminate,
            bytes_done: 0,
            transferred: 0,
            skipped: 0,
            failed: 0,
            files_seen: 0,
            defer_large,
            large_threshold: LARGE_THRESHOLD,
            pending_large: None,
            last_emitted_pct: u32::MAX, // force first emit
            last_emit_time: Instant::now(),
            indeterminate_tick: 0,
            last_speed_str: String::new(),
            last_bps: 0,
            speed_window_bytes: 0,
            speed_window_start: Instant::now(),
        }
    }

    fn note_bytes(&mut self, n: u64, now: Instant) {
        self.bytes_done = self.bytes_done.saturating_add(n);
        // Drift fix: source changed between scan and copy — expand the total
        // instead of clamping forever below 100%.
        if self.bytes_done > self.total_bytes && self.total_bytes > 0 {
            self.total_bytes = self.bytes_done;
        }

        self.speed_window_bytes = self.speed_window_bytes.saturating_add(n);
        let window_ms = now.saturating_duration_since(self.speed_window_start).as_millis();
        if window_ms >= SPEED_WINDOW_MS {
            let instant_bps = (self.speed_window_bytes as f64 / window_ms as f64 * 1000.0) as u64;
            if instant_bps > 0 {
                self.last_bps = if self.last_bps == 0 {
                    instant_bps
                } else {
                    (self.last_bps as f64 * 0.7 + instant_bps as f64 * 0.3) as u64
                };
                self.last_speed_str = fmt_speed(self.last_bps);
            }
            self.speed_window_bytes = 0;
            self.speed_window_start = now;
        }
    }

    /// Credit the remainder of a deferred large file.
    pub(crate) fn finalize_pending(&mut self) {
        if let Some(p) = self.pending_large.take() {
            let rest = p.size.saturating_sub(p.credited);
            if rest > 0 {
                self.note_bytes(rest, Instant::now());
            }
        }
    }

    fn current_pct(&mut self) -> u32 {
        if self.indeterminate {
            self.indeterminate_tick = (self.indeterminate_tick + 1) % 100;
            self.indeterminate_tick
        } else {
            overall_pct(self.bytes_done, self.total_bytes)
        }
    }

    fn maybe_emit(&mut self, name: String, pct: u32, now: Instant) -> Option<WarpProgress> {
        if pct != self.last_emitted_pct
            || now.saturating_duration_since(self.last_emit_time).as_millis()
                >= EMIT_MIN_INTERVAL_MS
        {
            self.last_emitted_pct = pct;
            self.last_emit_time = now;
            Some(WarpProgress {
                percentage: pct,
                current_file: name,
                speed: self.last_speed_str.clone(),
                files_done: self.files_seen,
                files_total: self.files_total_scan,
                indeterminate: self.indeterminate,
                bytes_per_sec: self.last_bps,
                bytes_done: self.bytes_done,
                total_bytes: self.total_bytes,
                active_workers: 0, // coordinator stamps real values before emitting
                shards_done: 0,
                shards_total: 0,
            })
        } else {
            None
        }
    }

    pub(crate) fn ingest(&mut self, line: &RoboLine, now: Instant) -> Option<WarpProgress> {
        match line {
            RoboLine::FileHeader { is_same, is_error, size, name } => {
                if self.defer_large {
                    self.finalize_pending();
                }
                self.files_seen += 1;
                if *is_error {
                    self.failed += 1;
                } else if *is_same {
                    self.skipped += 1;
                } else {
                    self.transferred += 1;
                }

                // Parity with the sequential engine: "Same"/error rows also
                // advance byte progress (a Same file IS done — nothing left to
                // copy for it); only genuinely-deferred large copies wait.
                let is_deferred = self.defer_large
                    && !*is_same
                    && !*is_error
                    && !self.indeterminate
                    && *size >= self.large_threshold;
                if is_deferred {
                    self.pending_large =
                        Some(PendingLarge { size: *size, name: name.clone(), credited: 0 });
                    let pct = self.current_pct();
                    return self.maybe_emit(name.clone(), pct, now);
                }

                self.note_bytes(*size, now);
                let pct = self.current_pct();
                self.maybe_emit(name.clone(), pct, now)
            }

            RoboLine::Percent(p) => {
                if !self.defer_large {
                    return None;
                }
                let pend = self.pending_large.as_mut()?;
                let p_clamped = p.clamp(0.0, 100.0);
                let want = (pend.size as f64 * p_clamped / 100.0) as u64;
                if want <= pend.credited {
                    return None; // regression/no-op — ignore like the sequential engine
                }
                let delta = want - pend.credited;
                pend.credited = want;
                let name = pend.name.clone();
                self.note_bytes(delta, now);
                let pct = if self.indeterminate {
                    self.indeterminate_tick
                } else {
                    overall_pct(self.bytes_done, self.total_bytes)
                };
                self.maybe_emit(name, pct, now)
            }

            RoboLine::Extra { size, name } => {
                // Parallel engine never runs Sync (hard gate), but handle Extra
                // so future callers and tests see live progress for deletes.
                self.files_seen += 1;
                self.transferred += 1;
                self.note_bytes(*size, now);
                let pct = self.current_pct();
                self.maybe_emit(format!("Deleting {}", name), pct, now)
            }

            RoboLine::Speed(bps) => {
                if self.last_speed_str.is_empty() {
                    self.last_speed_str = fmt_speed(*bps);
                }
                None
            }

            RoboLine::Skip => None,
        }
    }

    /// Undo a failed attempt's byte accounting before a retry re-runs it.
    pub(crate) fn revert_bytes(&mut self, n: u64) {
        self.bytes_done = self.bytes_done.saturating_sub(n);
    }
}

// — Shard execution primitives ----------------------------------------------

#[derive(Clone, Debug, Default)]
pub(crate) struct LocalCounters {
    pub transferred: u32,
    pub skipped: u32,
    pub failed: u32,
    pub seen: u32,
    pub counted_bytes: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct ShardOutcome {
    pub id: u64,
    pub transferred: u32,
    pub skipped: u32,
    pub failed: u32,
    pub counted_bytes: u64,
    pub exit_code: i32,
    pub had_exit_code: bool,
}

impl ShardOutcome {
    pub(crate) fn from_local(
        id: u64,
        l: &LocalCounters,
        exit_code: i32,
        had_exit_code: bool,
    ) -> Self {
        Self {
            id,
            transferred: l.transferred,
            skipped: l.skipped,
            failed: l.failed,
            counted_bytes: l.counted_bytes,
            exit_code,
            had_exit_code,
        }
    }
}

/// Robocopy arguments for one shard. Mirrors the sequential flag set minus
/// mode/throttle branches the coordinator handles explicitly (`/MOVE`, `/LEV:1`).
#[allow(dead_code)]
pub(crate) fn shard_args(
    src: &str,
    dst: &str,
    skip_conflict: bool,
    root_only: bool,
    move_mode: bool,
    mt: u32,
) -> Vec<String> {
    shard_args_with_filter(src, dst, skip_conflict, root_only, move_mode, mt, None)
}

pub(crate) fn shard_args_with_filter(
    src: &str,
    dst: &str,
    skip_conflict: bool,
    root_only: bool,
    move_mode: bool,
    mt: u32,
    filter: Option<&str>,
) -> Vec<String> {
    let mut args = vec![src.to_string(), dst.to_string()];
    if move_mode {
        args.push("/MOVE".to_string());
    }
    if skip_conflict {
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
        "/256".to_string(),
        "/XJ".to_string(), // exclude junctions (matches walk_dir symlink skip)
        "/XJD".to_string(),
        "/COPY:DAT".to_string(),
    ]);
    if root_only {
        args.push("/LEV:1".to_string());
    }
    if let Some(f) = filter {
        for pat in f.split([';', ',', ' ']) {
            let t = pat.trim();
            if t.is_empty() || t.len() > 100 || t.contains("..") || t.contains('\\') {
                continue;
            }
            if args.len() > 60 {
                break;
            }
            args.push("/XF".to_string());
            args.push(t.to_string());
            args.push("/XD".to_string());
            args.push(t.to_string());
        }
    }
    args.push(format!("/MT:{mt}"));
    args
}

/// Worker-count policy. `requested` comes from the UI: 0 = Auto, 2..=8 explicit.
///
/// Hard gates first (correctness/accuracy over speed):
///   - throttled transfers stay single-process — `/IPG` caps are per-process
///     and splitting them would make the cap inaccurate
///   - sync now uses two-phase parallel (phase 1: delete *EXTRA in parallel,
///     phase 2: copy in parallel, strictly sequential — 8 delete -> 8 copy,
///     never 4+4). Shard disjointness guarantees no cross-worker clobber.
///     Throttle still gates sync to single.
///
/// Explicit requests bypass the job-size heuristics (the user decided) but
/// never the hard gates. Auto applies medium caps: USB 2, network 3, local
/// clamp 2..=6 (never assume more threads = faster).
pub(crate) fn resolve_workers_for(
    requested: u8,
    _mode: &str,
    throttle: u32,
    shards: usize,
    total_files: u64,
    total_bytes: u64,
    usb: bool,
    network: bool,
) -> usize {
    if throttle > 0 || shards < 2 {
        return 1;
    }
    if requested > 1 {
        return (requested as usize).min(8);
    }
    if total_files < 400 || total_bytes < 256 * 1024 * 1024 {
        return 1;
    }
    if usb {
        2
    } else if network {
        3
    } else {
        std::thread::available_parallelism().map(|n| (n.get() / 2).clamp(2, 6)).unwrap_or(4)
    }
}

/// Files recovered by a retry attempt = previous failures minus remaining ones.
pub(crate) fn recovered_from_retry(prev_failed: u32, new_failed: u32) -> u32 {
    prev_failed.saturating_sub(new_failed)
}

// — Stream consumption --------------------------------------------------------

/// Parse one robocopy child's stdout. Updates the shard-local counters always,
/// and the shared display tracker when one is attached. Emission callbacks run
/// on the calling thread; keep them cheap (they do IPC).
pub(crate) fn consume_stream<R: BufRead>(
    reader: R,
    is_cancelled: &dyn Fn() -> bool,
    tracker: Option<&Mutex<Tracker>>,
    local: &mut LocalCounters,
    mut on_progress: impl FnMut(WarpProgress),
    mut on_error_line: impl FnMut(String),
) {
    for line in reader.lines().flatten() {
        if is_cancelled() {
            break;
        }
        match parse_line(&line) {
            RoboLine::FileHeader { is_same, is_error, size, name } => {
                local.seen += 1;
                if is_error {
                    local.failed += 1;
                    on_error_line(name.clone());
                } else if is_same {
                    local.skipped += 1;
                } else {
                    local.transferred += 1;
                }
                // Parallel mode counts every header's bytes immediately --
                // counted_bytes therefore equals what the tracker was fed and
                // is exactly what a retry must revert.
                local.counted_bytes = local.counted_bytes.saturating_add(size);
                if let Some(t) = tracker {
                    let mut g = t.lock().unwrap_or_else(|e| e.into_inner());
                    let parsed = RoboLine::FileHeader { is_same, is_error, size, name };
                    if let Some(p) = g.ingest(&parsed, Instant::now()) {
                        drop(g);
                        on_progress(p);
                    }
                }
            }
            RoboLine::Extra { size, name } => {
                local.seen += 1;
                local.transferred += 1;
                local.counted_bytes = local.counted_bytes.saturating_add(size);
                if let Some(t) = tracker {
                    let mut g = t.lock().unwrap_or_else(|e| e.into_inner());
                    let parsed = RoboLine::Extra { size, name: name.clone() };
                    if let Some(p) = g.ingest(&parsed, Instant::now()) {
                        drop(g);
                        on_progress(p);
                    }
                }
            }
            RoboLine::Percent(_) | RoboLine::Speed(_) => {
                if let Some(t) = tracker {
                    let parsed = parse_line(&line);
                    let mut g = t.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(p) = g.ingest(&parsed, Instant::now()) {
                        drop(g);
                        on_progress(p);
                    }
                }
            }
            RoboLine::Skip => {}
        }
    }
}

// — Tests ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn t(offset_ms: u64) -> Instant {
        Instant::now().checked_add(Duration::from_millis(offset_ms)).unwrap()
    }

    fn hdr(size: u64) -> RoboLine {
        RoboLine::FileHeader { is_same: false, is_error: false, size, name: format!("f{size}.bin") }
    }

    #[test]
    fn small_files_count_bytes_and_emit_monotonically() {
        let mut tr = Tracker::new(1000, 10, false, false);
        assert!(tr.ingest(&hdr(300), t(0)).is_some(), "first event always emits");
        let p = tr.ingest(&hdr(300), t(10)).expect("pct changed 30->60");
        assert_eq!(p.percentage, 60);
        assert_eq!(tr.bytes_done, 600);
        assert_eq!(tr.transferred, 2);
        // No pct change and <150ms -> suppressed.
        assert!(tr.ingest(&hdr(1), t(20)).is_none());
    }

    #[test]
    fn same_and_error_rows_still_advance_bytes_like_sequential() {
        let mut tr = Tracker::new(1000, 10, false, false);
        tr.ingest(&hdr(500), t(0));
        let same =
            RoboLine::FileHeader { is_same: true, is_error: false, size: 250, name: "s".into() };
        tr.ingest(&same, t(5)).unwrap();
        assert_eq!(tr.skipped, 1);
        assert_eq!(tr.bytes_done, 750);
        let err =
            RoboLine::FileHeader { is_same: false, is_error: true, size: 0, name: "e".into() };
        tr.ingest(&err, t(10));
        assert_eq!(tr.failed, 1);
    }

    #[test]
    fn drift_expands_total_instead_of_clamping_forever() {
        let mut tr = Tracker::new(100, 1, false, false);
        tr.ingest(&hdr(200), t(0));
        assert_eq!(tr.total_bytes, 200, "total grows to observed bytes");
        assert!(tr.bytes_done <= tr.total_bytes);
    }

    #[test]
    fn indeterminate_mode_pulses_without_totals() {
        let mut tr = Tracker::new(0, 0, true, false);
        let p = tr.ingest(&hdr(0), t(0)).unwrap();
        assert!(p.indeterminate);
        assert!(p.bytes_per_sec == 0 || p.percentage < 100);
    }

    #[test]
    fn deferred_large_file_tracks_percent_then_finalizes_full_size() {
        let mut tr = Tracker::new(2000, 2, false, true);

        // 1500-byte "large" file — the gate is lowered to 1000 directly so the
        // state machine is exercised without synthetic gigabyte numbers.
        // (The follow-up 500-byte file then stays under the gate.)
        tr.large_threshold = 1000;
        let big = RoboLine::FileHeader {
            is_same: false,
            is_error: false,
            size: 1500,
            name: "big".into(),
        };
        let p0 = tr.ingest(&big, t(0)).unwrap();
        assert_eq!(tr.bytes_done, 0, "deferred: nothing counted yet");

        let half = tr.ingest(&RoboLine::Percent(50.0), t(50)).unwrap();
        assert_eq!(half.bytes_done, 750);

        // Regression (percent going down) must be ignored.
        assert!(tr.ingest(&RoboLine::Percent(10.0), t(60)).is_none());
        assert_eq!(tr.bytes_done, 750);

        tr.finalize_pending();
        assert_eq!(tr.bytes_done, 1500, "finalize credits the remainder");

        // Next file counts normally.
        tr.ingest(&hdr(500), t(100));
        assert_eq!(tr.bytes_done, 2000);
        assert_eq!(p0.files_total, 2);
    }

    #[test]
    fn parallel_mode_ignores_percent_lines() {
        let mut tr = Tracker::new(1000, 2, false, false);
        tr.ingest(&hdr(400), t(0));
        assert!(tr.ingest(&RoboLine::Percent(90.0), t(5)).is_none());
        assert_eq!(tr.bytes_done, 400);
    }

    #[test]
    fn speed_ewma_forms_after_first_window() {
        let mut tr = Tracker::new(u64::MAX / 2, 100, false, false);
        tr.ingest(&hdr(50_000_000), t(0)); // 50MB instantly -> ~huge bps after 400ms window closes
        let p = tr.ingest(&hdr(50_000_000), t(450));
        assert!(p.is_some());
        assert!(!tr.last_speed_str.is_empty(), "speed string populated after window");
        assert!(tr.last_bps > 0);
    }

    #[test]
    fn resolve_workers_gates_sync_throttle_and_small_jobs() {
        let big_files = 10_000u64;
        let big_bytes = 4u64 * 1024 * 1024 * 1024;
        // Hard gates win regardless of request.
        // Sync now allows parallel via two-phase delete->copy (throttle still gates).
        assert_eq!(resolve_workers_for(8, "sync", 0, 5, big_files, big_bytes, false, false), 8);
        assert_eq!(resolve_workers_for(8, "copy", 25, 5, big_files, big_bytes, false, false), 1);
        assert_eq!(resolve_workers_for(8, "copy", 0, 1, big_files, big_bytes, false, false), 1);
        // Explicit request honored on eligible jobs.
        assert_eq!(resolve_workers_for(4, "copy", 0, 5, big_files, big_bytes, false, false), 4);
        assert_eq!(
            resolve_workers_for(99, "copy", 0, 5, big_files, big_bytes, false, false),
            8,
            "clamped"
        );
        // Auto: medium caps.
        assert_eq!(resolve_workers_for(0, "copy", 0, 5, big_files, big_bytes, true, false), 2);
        assert_eq!(resolve_workers_for(0, "copy", 0, 5, big_files, big_bytes, false, true), 3);
        let w = resolve_workers_for(0, "copy", 0, 5, big_files, big_bytes, false, false);
        assert!((2..=6).contains(&w));
        // Auto skips small jobs entirely.
        assert_eq!(resolve_workers_for(0, "copy", 0, 5, 10, big_bytes, false, false), 1);
        assert_eq!(resolve_workers_for(0, "copy", 0, 5, big_files, 1024, false, false), 1);
    }

    #[test]
    fn shard_args_match_sequential_flag_set() {
        let plain = shard_args("C:\\s", "C:\\d", false, false, false, 8);
        assert_eq!(plain[0], "C:\\s");
        assert_eq!(plain[1], "C:\\d");
        for f in [
            "/E",
            "/NP",
            "/R:3",
            "/W:5",
            "/BYTES",
            "/NJH",
            "/NJS",
            "/256",
            "/XJ",
            "/XJD",
            "/COPY:DAT",
        ] {
            assert!(plain.iter().any(|a| a == f), "missing {f}");
        }
        assert!(plain.iter().any(|a| a == "/MT:8"));
        assert!(!plain.iter().any(|a| a == "/LEV:1"));
        assert!(!plain.iter().any(|a| a == "/MOVE"));

        let root = shard_args("C:\\s", "C:\\d", true, true, true, 4);
        assert!(root.iter().any(|a| a == "/LEV:1"));
        assert!(root.iter().any(|a| a == "/MOVE"));
        assert!(root.iter().any(|a| a == "/XO") && root.iter().any(|a| a == "/XN"));
    }

    #[test]
    fn retry_recovery_never_negative() {
        assert_eq!(recovered_from_retry(5, 2), 3);
        assert_eq!(recovered_from_retry(2, 5), 0);
        assert_eq!(recovered_from_retry(0, 0), 0);
    }
}
