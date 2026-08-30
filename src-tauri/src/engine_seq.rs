use std::io::{BufRead, BufReader};
use std::time::Instant;
use tauri::{Emitter, Window};

use crate::parser::{parse_line, RoboLine};
use crate::preflight::{ensure_free_space, is_path_on_usb};
use crate::progress::{fmt_speed, ipg_for_throttle, overall_pct};
use crate::{robocopy_cmd, TransferControl, WarpProgress, WarpSummary, SEQ_CHILD_ID};

pub fn parse_filter(filter: Option<&str>) -> Vec<String> {
    let Some(s) = filter else { return vec![] };
    let mut out = Vec::new();
    for part in s.split([';', ',', ' ']) {
        let t = part.trim();
        if t.is_empty() || t.len() > 100 || t.contains("..") || t.contains('\\') {
            continue;
        }
        out.push(t.to_string());
        if out.len() >= 20 {
            break;
        }
    }
    out
}

pub fn scan(source: &str, destination: &str, mode: &str) -> (u64, u32) {
    let is_sync = mode == "sync";
    let mut args = vec![
        source.to_string(),
        destination.to_string(),
        "/L".to_string(),
        "/E".to_string(),
        "/BYTES".to_string(),
        "/NJH".to_string(),
        "/NJS".to_string(),
        "/NP".to_string(),
    ];
    if is_sync {
        args.push("/MIR".to_string());
    }
    let out = match robocopy_cmd().args(&args).output() {
        Ok(o) => o,
        Err(_) => return (0, 0),
    };

    let mut total_bytes = 0u64;
    let mut total_files = 0u32;
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        match parse_line(line) {
            RoboLine::FileHeader { size, is_error: false, .. } => {
                total_bytes += size;
                total_files += 1;
            }
            RoboLine::Extra { size, .. } if is_sync => {
                total_bytes = total_bytes.saturating_add(size);
                total_files += 1;
            }
            _ => {}
        }
    }
    (total_bytes, total_files)
}

/// The sequential transfer pipeline (scan -> copy -> verify). Runs on the
/// blocking thread pool via `spawn_blocking`.
#[allow(clippy::too_many_arguments)]
pub fn warp_file_op_sync(
    window: Window,
    control: &TransferControl,
    source: String,
    destination: String,
    effective_dest: String,
    mode: String,
    conflict: String,
    throttle: u32,
    verify: bool,
    filter: Option<String>,
    quick_bytes: u64,
    quick_files: u32,
    workers: Option<u8>,
) -> Result<WarpSummary, String> {
    let (mut total_bytes, total_files_scan) = if mode == "sync" {
        scan(&source, &effective_dest, &mode)
    } else {
        (quick_bytes, quick_files)
    };
    let indeterminate = total_bytes == 0;

    ensure_free_space(&destination, &effective_dest, total_bytes)?;

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
        "/R:0".to_string(),
        "/W:0".to_string(),
        "/BYTES".to_string(),
        "/NJH".to_string(),
        "/NJS".to_string(),
        "/256".to_string(),
        "/XJ".to_string(),
        "/XJD".to_string(),
        "/COPY:DAT".to_string(),
    ]);

    for pat in parse_filter(filter.as_deref()) {
        args.push("/XF".to_string());
        args.push(pat.clone());
        args.push("/XD".to_string());
        args.push(pat);
    }

    let is_usb_source = is_path_on_usb(&source);
    let is_usb_dest = is_path_on_usb(&effective_dest);
    let is_usb = is_usb_source || is_usb_dest;
    let is_large = total_bytes > 256 * 1024 * 1024;

    if let Some(ipg) = ipg_for_throttle(throttle) {
        if throttle >= 25 {
            let per_thread_ipg = (ipg / 2).max(1);
            args.push(format!("/IPG:{}", per_thread_ipg));
            args.push("/MT:4".to_string());
        } else {
            args.push(format!("/IPG:{ipg}"));
        }
        if is_large && throttle < 25 {
            args.push("/Z".to_string());
        }
    } else if is_usb {
        args.push("/MT:4".to_string());
        args.push("/Z".to_string());
    } else if is_large {
        let mt = match workers {
            Some(w) if w >= 8 => 128,
            Some(w) if w >= 4 => 64,
            Some(w) if w >= 2 => 32,
            _ => 32,
        };
        args.push(format!("/MT:{}", mt));
        args.push("/J".to_string());
    } else {
        let mt = match workers {
            Some(w) if w >= 8 => 128,
            Some(w) if w >= 4 => 64,
            Some(w) if w >= 2 => 32,
            _ => 64,
        };
        args.push(format!("/MT:{}", mt));
    }

    let mut child =
        robocopy_cmd().args(&args).spawn().map_err(|e| format!("Failed to start robocopy: {e}"))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    control.register(SEQ_CHILD_ID, child);

    if let Some(stderr) = stderr {
        let win2 = window.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().flatten() {
                let t = line.trim().to_string();
                if !t.is_empty() {
                    let _ = win2.emit("warp-error", t.clone());
                    crate::log_event(&format!("robocopy stderr: {}", t));
                }
            }
        });
    }

    let start = Instant::now();
    let mut summary = WarpSummary {
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
        workers_used: workers.unwrap_or(0).max(1) as u32,
        retried_ok: 0,
    };

    let mut bytes_done: u64 = 0;
    let mut last_emitted: u32 = 0;
    let mut last_speed_str = String::new();
    let mut last_bps: u64 = 0;
    let mut files_done_count: u32 = 0;

    let mut speed_window_bytes: u64 = 0;
    let mut speed_window_start = Instant::now();

    let mut indeterminate_tick: u32 = 0;
    let mut last_emit_time = Instant::now();

    let mut pending_large: Option<(u64, u64, String, f64)> = None;
    const LARGE_THRESHOLD: u64 = 10 * 1024 * 1024;

    let finalize_pending =
        |bytes_done: &mut u64,
         summary: &mut WarpSummary,
         total_bytes: &mut u64,
         pending: &mut Option<(u64, u64, String, f64)>| {
            if let Some((sz, before, _name, _pct)) = pending.take() {
                let new_done = before.saturating_add(sz);
                if new_done > *total_bytes && *total_bytes > 0 {
                    *total_bytes = new_done;
                }
                *bytes_done = new_done;
                summary.bytes_transferred = *bytes_done;
            }
        };

    if let Some(stdout) = stdout {
        for line in BufReader::new(stdout).lines().flatten() {
            if control.is_cancelled() {
                summary.cancelled = true;
                break;
            }

            match parse_line(&line) {
                RoboLine::FileHeader { is_same, is_error, size, name } => {
                    finalize_pending(
                        &mut bytes_done,
                        &mut summary,
                        &mut total_bytes,
                        &mut pending_large,
                    );

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

                    let is_large_transfer =
                        !is_same && !is_error && !indeterminate && size >= LARGE_THRESHOLD;
                    if is_large_transfer {
                        pending_large = Some((size, bytes_done, name.clone(), 0.0));
                        let pct = overall_pct(bytes_done, total_bytes);
                        if pct != last_emitted || last_emit_time.elapsed().as_millis() >= 150 {
                            last_emitted = pct;
                            last_emit_time = Instant::now();
                            let _ = window.emit(
                                "warp-progress",
                                WarpProgress {
                                    percentage: pct,
                                    current_file: name.clone(),
                                    speed: last_speed_str.clone(),
                                    files_done: files_done_count,
                                    files_total: total_files_scan,
                                    indeterminate,
                                    bytes_per_sec: last_bps,
                                    bytes_done,
                                    total_bytes,
                                    active_workers: 1,
                                    shards_done: 0,
                                    shards_total: 0,
                                },
                            );
                        }
                    } else {
                        bytes_done = bytes_done.saturating_add(size);
                        if bytes_done > total_bytes && total_bytes > 0 {
                            total_bytes = bytes_done;
                        }
                        summary.bytes_transferred = bytes_done;

                        speed_window_bytes = speed_window_bytes.saturating_add(size);
                        let window_ms = speed_window_start.elapsed().as_millis() as u64;
                        if window_ms >= 400 {
                            let instant_bps =
                                (speed_window_bytes as f64 / window_ms as f64 * 1000.0) as u64;
                            if instant_bps > 0 {
                                last_bps = if last_bps == 0 {
                                    instant_bps
                                } else {
                                    (last_bps as f64 * 0.7 + instant_bps as f64 * 0.3) as u64
                                };
                                last_speed_str = fmt_speed(last_bps);
                            }
                            speed_window_bytes = 0;
                            speed_window_start = Instant::now();
                        }

                        let pct = if indeterminate {
                            indeterminate_tick = (indeterminate_tick + 1) % 100;
                            indeterminate_tick
                        } else {
                            overall_pct(bytes_done, total_bytes)
                        };

                        let should_emit =
                            pct != last_emitted || last_emit_time.elapsed().as_millis() >= 150;
                        if should_emit {
                            last_emitted = pct;
                            last_emit_time = Instant::now();
                            let _ = window.emit(
                                "warp-progress",
                                WarpProgress {
                                    percentage: pct,
                                    current_file: name,
                                    speed: last_speed_str.clone(),
                                    files_done: files_done_count,
                                    files_total: total_files_scan,
                                    indeterminate,
                                    bytes_per_sec: last_bps,
                                    bytes_done,
                                    total_bytes,
                                    active_workers: 1,
                                    shards_done: 0,
                                    shards_total: 0,
                                },
                            );
                        }
                    }
                }

                RoboLine::Extra { size, name } => {
                    finalize_pending(
                        &mut bytes_done,
                        &mut summary,
                        &mut total_bytes,
                        &mut pending_large,
                    );
                    summary.total_files += 1;
                    files_done_count += 1;
                    bytes_done = bytes_done.saturating_add(size);
                    if bytes_done > total_bytes && total_bytes > 0 {
                        total_bytes = bytes_done;
                    }
                    summary.bytes_transferred = bytes_done;
                    speed_window_bytes = speed_window_bytes.saturating_add(size);
                    let window_ms = speed_window_start.elapsed().as_millis() as u64;
                    if window_ms >= 400 {
                        let instant_bps =
                            (speed_window_bytes as f64 / window_ms as f64 * 1000.0) as u64;
                        if instant_bps > 0 {
                            last_bps = if last_bps == 0 {
                                instant_bps
                            } else {
                                (last_bps as f64 * 0.7 + instant_bps as f64 * 0.3) as u64
                            };
                            last_speed_str = fmt_speed(last_bps);
                        }
                        speed_window_bytes = 0;
                        speed_window_start = Instant::now();
                    }
                    let pct = if indeterminate {
                        indeterminate_tick = (indeterminate_tick + 1) % 100;
                        indeterminate_tick
                    } else {
                        overall_pct(bytes_done, total_bytes)
                    };
                    let should_emit =
                        pct != last_emitted || last_emit_time.elapsed().as_millis() >= 150;
                    if should_emit {
                        last_emitted = pct;
                        last_emit_time = Instant::now();
                        let _ = window.emit(
                            "warp-progress",
                            WarpProgress {
                                percentage: pct,
                                current_file: format!("Deleting {}", name),
                                speed: last_speed_str.clone(),
                                files_done: files_done_count,
                                files_total: total_files_scan,
                                indeterminate,
                                bytes_per_sec: last_bps,
                                bytes_done,
                                total_bytes,
                                active_workers: 1,
                                shards_done: 0,
                                shards_total: 0,
                            },
                        );
                    }
                }

                RoboLine::Percent(p) => {
                    if let Some((sz, before, ref name, ref mut last_p)) = pending_large {
                        *last_p = p;
                        let file_part = (sz as f64 * (p / 100.0)) as u64;
                        let cur_done = before.saturating_add(file_part);
                        summary.bytes_transferred = cur_done;

                        speed_window_bytes = speed_window_bytes.saturating_add(file_part);
                        let window_ms = speed_window_start.elapsed().as_millis() as u64;
                        if window_ms >= 400 {
                            let instant_bps =
                                (speed_window_bytes as f64 / window_ms as f64 * 1000.0) as u64;
                            if instant_bps > 0 {
                                last_bps = if last_bps == 0 {
                                    instant_bps
                                } else {
                                    (last_bps as f64 * 0.7 + instant_bps as f64 * 0.3) as u64
                                };
                                last_speed_str = fmt_speed(last_bps);
                            }
                            speed_window_bytes = 0;
                            speed_window_start = Instant::now();
                        }

                        let pct = overall_pct(cur_done, total_bytes);
                        let should_emit =
                            pct != last_emitted || last_emit_time.elapsed().as_millis() >= 150;
                        if should_emit {
                            last_emitted = pct;
                            last_emit_time = Instant::now();
                            let _ = window.emit(
                                "warp-progress",
                                WarpProgress {
                                    percentage: pct,
                                    current_file: format!("{} ({:.1}%)", name, p),
                                    speed: last_speed_str.clone(),
                                    files_done: files_done_count,
                                    files_total: total_files_scan,
                                    indeterminate: false,
                                    bytes_per_sec: last_bps,
                                    bytes_done: cur_done,
                                    total_bytes,
                                    active_workers: 1,
                                    shards_done: 0,
                                    shards_total: 0,
                                },
                            );
                        }
                    }
                }

                RoboLine::Speed(bps) => {
                    if last_bps == 0 {
                        last_bps = bps;
                        last_speed_str = fmt_speed(bps);
                    }
                }

                RoboLine::Skip => {}
            }
        }
    }

    finalize_pending(&mut bytes_done, &mut summary, &mut total_bytes, &mut pending_large);

    let (code, was_terminated_without_code) = match control.take(SEQ_CHILD_ID) {
        Some(ref mut child) => match child.wait() {
            Ok(status) => match status.code() {
                Some(v) => (v, false),
                None => (-1, true),
            },
            Err(_) => (-1, true),
        },
        None => (0, false),
    };

    summary.duration_ms = start.elapsed().as_millis() as u64;
    summary.error_code = code;

    if summary.cancelled {
        crate::log_event(&format!(
            "cancelled after {} ms, {}/{} files, {} bytes",
            summary.duration_ms,
            summary.transferred,
            summary.total_files,
            summary.bytes_transferred
        ));
        return Ok(summary);
    }

    if was_terminated_without_code {
        summary.error_message =
            "Transfer terminated unexpectedly (no exit code — process was killed)".to_string();
        crate::log_event("terminated without exit code");
        return Ok(summary);
    }

    if code < 8 {
        crate::log_event(&format!(
            "success code={} transferred={} skipped={} failed={} bytes={} verified={}",
            code,
            summary.transferred,
            summary.skipped,
            summary.failed,
            summary.bytes_transferred,
            summary.verified
        ));

        if mode == "move" && summary.failed == 0 && !summary.cancelled {
            crate::trash::log_trash(&mode, &source, &effective_dest, vec![]);
            crate::remove_empty_dirs(std::path::Path::new(&source));
        }
        if mode == "sync" && summary.failed == 0 && !summary.cancelled {
            crate::trash::log_trash(&mode, &source, &effective_dest, vec![]);
        }

        if verify && mode != "move" && summary.failed == 0 {
            let _ = window.emit("warp-verifying", ());
            summary.verify_mismatches = crate::verify::verify_transfer(&source, &effective_dest);
            summary.verified = true;
            crate::log_event(&format!("verify mismatches={}", summary.verify_mismatches));
        }

        Ok(summary)
    } else {
        summary.error_message = crate::robocopy_exit_message(code)
            .unwrap_or_else(|| format!("Transfer failed (exit code {})", code));
        crate::log_event(&format!("failed code={} msg={}", code, summary.error_message));
        Ok(summary)
    }
}
