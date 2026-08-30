# 006 — Light Mode + Scale Adjuster

**Status:** Implemented — 2026-08-31 (`ui.svelte.ts` theme/scale, `app.css` data-theme, `OptionsPanel` controls)  
**Related:** `src/app.css` (design tokens, `prefers-color-scheme`), `src/lib/stores/ui.svelte.ts`, `src/lib/components/OptionsPanel.svelte`

## Summary

Add two frontend-only preferences that do not touch the Rust engine:

- **Light Mode** — System / Dark / Light toggle, persisted, applied via CSS variables.
- **Scale Adjuster** — 1.0x to ~1.5x slider, default and minimum is **1.0x (current size)**. Larger only, never smaller, so current layout stays the baseline and accessibility users can scale up.

## Light Mode

### Where

`src/app.css` already defines dark as default and a light block gated by `@media (prefers-color-scheme: light)` (`--glass-bg`, `--text-primary`, etc. `app.css:195`). Keep dark as default.

### Control

New segmented control (same style as Workers) in header or `OptionsPanel` — 3 states: `System | Dark | Light`.

- `System` — follow OS `prefers-color-scheme`
- `Dark` — force `data-theme="dark"` on `<html>`
- `Light` — force `data-theme="light"` on `<html>`

### Persist

`localStorage` keys `warp-theme` + `warp-scale` separate. `ui.svelte.ts` `theme = $state<"system"|"dark"|"light">("system")` loads on `onMount`, writes on change. No backend needed.

### Apply

- CSS only: `:root` holds dark tokens, `[data-theme="light"]` overrides, `[data-theme="dark"]` forces dark, otherwise media query for System.
- Example tokens to override in light: `--glass-bg: rgba(255,255,255,0.78)`, `--glass-border: rgba(0,0,0,0.08)`, `--text-primary: rgba(0,0,0,0.92)`, etc. (already in `app.css:198`).
- Keep glass blur `48px` and `TrafficLights` contrast — test on Win10/11 light title bar vs dark WebView2 `transparent` background.

### Edge

Tauri `transparent` window + WebView2 — light needs `background: var(--surface-0)` opaque fallback for screenshot. Keep `ResultCards` green/red accessible on light (check contrast).

## Scale Adjuster

### Control

New row `Size` in `OptionsPanel` (or header) — slider `1.0x` to `1.5x` step `0.05x`, display `100%` to `150%`, plus `Reset` to `1.0x`. Default `1.0x`.

- Minimum is **1.0x** — current size is the smallest, never shrinks. Only scales up for accessibility / large displays.
- Maximum `1.5x` (150%) — cap to avoid overflow at `800×1100` max window. Can raise to `1.75x` later if needed.

### Persist

`localStorage` `warp-scale` number `1.0` to `1.5`. `ui.svelte.ts` `scale = $state(1.0)` loads on mount.

### Apply (recommended)

`html { font-size: calc(16px * var(--scale, 1)) }` where `--scale` is set on `<html>` via `document.documentElement.style.setProperty('--scale', scale)`.

- All text uses `rem`, spacing uses `em` or `rem`, so padding/gap scales with text — layout stays proportional, no blur like `transform: scale()`.
- `app.css` already uses `rem` for `font-size` and `em` for `letter-spacing`, so minimal changes. Convert any `px` gaps that should scale to `rem` (e.g., `gap: 14px` → `gap: 0.875rem`).

Alternative considered: `transform: scale()` on `.shell` — rejected (blurs, breaks `data-drop` hit testing with `devicePixelRatio` in `+page.svelte:66`).

### Range and Accessibility

- Text at `1.0x` = current `12px` labels, `14px` body. At `1.5x` = `18px` / `21px` — still fits `500px` shell with `overflow-y: auto`.
- Respect `prefers-reduced-motion` for any transition on scale.
- Test `ProgressCard` live list (5 recent files) and `ResultCards` stats at `1.5x` — ensure no clipping, `min-height: 100vh` scroll holds.

## No Backend Changes

Both are pure CSS + `localStorage`. No `src-tauri` changes, no new Tauri commands.

## Next Steps (upon approval)

1. Add `theme` + `scale` to `ui.svelte.ts` with `localStorage` load/save.
2. Add `data-theme` + `--scale` setters in `+page.svelte` `onMount`.
3. Add UI controls in `OptionsPanel` (or header) and `app.css` light overrides.
4. Manual test at `1.0x` and `1.5x` on Win10/11 dark/light.
