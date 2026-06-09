"use client";

import { motion, useInView } from "framer-motion";
import { useRef } from "react";

/* ── Mini mock-ups rendered as SVG/HTML inside each card ─────────────────── */

function QueueMockup() {
  const items = [
    { src: "Photos", dest: "Backup", mode: "copy" },
    { src: "Videos", dest: "External", mode: "copy · verify" },
    { src: "Documents", dest: "NAS", mode: "copy · 25 MB/s" },
  ];
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6, marginTop: 4 }}>
      {items.map((item, i) => (
        <div
          key={i}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            padding: "7px 10px",
            borderRadius: 8,
            background: "rgba(255,255,255,0.04)",
            border: "1px solid rgba(255,255,255,0.06)",
          }}
        >
          <span
            style={{
              fontSize: 9,
              fontWeight: 700,
              color: "var(--text-tertiary)",
              width: 14,
              flexShrink: 0,
            }}
          >
            {i + 1}
          </span>
          <div style={{ flex: 1, minWidth: 0 }}>
            <p style={{ fontSize: 11, color: "var(--text-primary)", margin: 0, fontWeight: 500 }}>
              {item.src} → {item.dest}
            </p>
            <p style={{ fontSize: 9, color: "var(--text-tertiary)", margin: "2px 0 0" }}>
              {item.mode}
            </p>
          </div>
        </div>
      ))}
      <div
        style={{
          padding: "9px 14px",
          borderRadius: 10,
          background: "var(--accent)",
          color: "#fff",
          fontSize: 12,
          fontWeight: 600,
          textAlign: "center",
          boxShadow: "0 2px 12px rgba(10,132,255,0.35)",
          marginTop: 4,
        }}
      >
        Run Queue (3 jobs)
      </div>
    </div>
  );
}

function ProgressMockup() {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8, marginTop: 4 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <div
          style={{
            width: 12,
            height: 12,
            borderRadius: "50%",
            border: "2px solid rgba(10,132,255,0.3)",
            borderTopColor: "var(--accent)",
            flexShrink: 0,
            animation: "spin 1.2s linear infinite",
          }}
        />
        <span style={{ fontSize: 11, color: "var(--text-secondary)", flex: 1 }}>
          holiday_2024.mp4
        </span>
        <span style={{ fontSize: 12, fontWeight: 600, color: "var(--accent)" }}>
          842 MB/s
        </span>
      </div>
      <div style={{ height: 3, background: "rgba(255,255,255,0.06)", borderRadius: 100, overflow: "hidden" }}>
        <div
          className="shimmer"
          style={{ height: "100%", width: "68%", borderRadius: 100 }}
        />
      </div>
      <div style={{ display: "flex", justifyContent: "space-between" }}>
        <span style={{ fontSize: 10, color: "var(--text-tertiary)" }}>
          847 / 1,204 files · 4m 12s left
        </span>
        <span style={{ fontSize: 10, color: "var(--text-tertiary)" }}>68%</span>
      </div>
    </div>
  );
}

function VerifyMockup() {
  return (
    <div
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 8,
        padding: "8px 14px",
        borderRadius: 10,
        background: "rgba(48,209,88,0.12)",
        border: "1px solid rgba(48,209,88,0.25)",
        marginTop: 8,
      }}
    >
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
        <path d="M2.5 7l3 3 6-6" stroke="var(--green)" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"/>
      </svg>
      <span style={{ fontSize: 12, fontWeight: 600, color: "var(--green)" }}>
        Verified — all files match
      </span>
    </div>
  );
}

function SpeedMockup() {
  const opts = ["Unlimited", "100 MB/s", "25 MB/s", "5 MB/s", "Custom"];
  return (
    <div
      style={{
        display: "inline-flex",
        gap: 2,
        padding: 3,
        background: "rgba(255,255,255,0.05)",
        border: "1px solid var(--glass-border)",
        borderRadius: 9,
        flexWrap: "wrap",
        marginTop: 8,
      }}
    >
      {opts.map((o, i) => (
        <div
          key={o}
          style={{
            padding: "4px 10px",
            borderRadius: 7,
            fontSize: 11,
            fontWeight: 600,
            background: i === 2 ? "rgba(255,255,255,0.16)" : "transparent",
            color: i === 2 ? "var(--text-primary)" : "var(--text-secondary)",
            boxShadow: i === 2 ? "0 1px 3px rgba(0,0,0,0.35)" : "none",
          }}
        >
          {o}
        </div>
      ))}
    </div>
  );
}

function PresetMockup() {
  const presets = ["Photos to Backup", "Videos to NAS", "Work to External"];
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6, marginTop: 4 }}>
      {presets.map((name) => (
        <div
          key={name}
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "8px 12px",
            borderRadius: 8,
            background: "rgba(255,255,255,0.04)",
            border: "1px solid rgba(255,255,255,0.06)",
            cursor: "pointer",
          }}
        >
          <span style={{ fontSize: 12, fontWeight: 500, color: "var(--text-primary)" }}>
            {name}
          </span>
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path d="M4.5 2.5l4 3.5-4 3.5" stroke="var(--accent)" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"/>
          </svg>
        </div>
      ))}
    </div>
  );
}

function SyncMockup() {
  return (
    <div
      style={{
        padding: "16px",
        borderRadius: 12,
        background: "#1c1c1e",
        border: "1px solid rgba(255,255,255,0.1)",
        marginTop: 4,
        maxWidth: 300,
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 10 }}>
        <div
          style={{
            width: 32,
            height: 32,
            borderRadius: 8,
            background: "rgba(255,69,58,0.15)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          <svg width="15" height="15" viewBox="0 0 20 20" fill="none">
            <path d="M10 3l7.5 13H2.5L10 3z" stroke="var(--red)" strokeWidth="1.5" fill="none" strokeLinejoin="round"/>
            <path d="M10 8v4m0 2.5v.5" stroke="var(--red)" strokeWidth="1.5" strokeLinecap="round"/>
          </svg>
        </div>
        <div>
          <p style={{ fontSize: 13, fontWeight: 600, color: "var(--text-primary)", margin: 0 }}>
            Sync will delete files
          </p>
          <p style={{ fontSize: 10, color: "var(--text-tertiary)", margin: "2px 0 0" }}>
            This cannot be undone
          </p>
        </div>
      </div>
      <div style={{ display: "flex", gap: 8 }}>
        <div
          style={{
            flex: 1,
            padding: "7px",
            borderRadius: 8,
            border: "1px solid rgba(255,255,255,0.1)",
            textAlign: "center",
            fontSize: 11,
            fontWeight: 500,
            color: "var(--text-secondary)",
          }}
        >
          Cancel
        </div>
        <div
          style={{
            flex: 1,
            padding: "7px",
            borderRadius: 8,
            background: "var(--red)",
            textAlign: "center",
            fontSize: 11,
            fontWeight: 600,
            color: "white",
          }}
        >
          Sync & Delete
        </div>
      </div>
    </div>
  );
}

/* ── Bento card data ─────────────────────────────────────────────────────── */

const cards = [
  {
    col: "col-span-2",
    accent: "var(--accent)",
    accentBg: "rgba(10,132,255,0.08)",
    label: "TRANSFER QUEUE",
    title: "Stack multiple jobs. Walk away.",
    body: "Add as many source/destination pairs as you need, then run the queue. Warp handles them back-to-back with a live per-job progress view.",
    mockup: <QueueMockup />,
  },
  {
    col: "col-span-1",
    accent: "var(--accent)",
    accentBg: "rgba(10,132,255,0.06)",
    label: "LIVE PROGRESS",
    title: "See exactly what's moving.",
    body: "Per-file live list, ETA from actual throughput, and a real overall percentage — not a guess.",
    mockup: <ProgressMockup />,
  },
  {
    col: "col-span-1",
    accent: "var(--green)",
    accentBg: "rgba(48,209,88,0.06)",
    label: "VERIFY MODE",
    title: "Confirm every file arrived.",
    body: "After copy or sync, Warp re-compares source and destination. Green check or exact mismatch count.",
    mockup: <VerifyMockup />,
  },
  {
    col: "col-span-1",
    accent: "var(--orange)",
    accentBg: "rgba(255,159,10,0.06)",
    label: "BANDWIDTH THROTTLE",
    title: "Copy without hogging your disk.",
    body: "Cap at 5 / 25 / 100 MB/s or set any custom value. Useful when transferring in the background.",
    mockup: <SpeedMockup />,
  },
  {
    col: "col-span-1",
    accent: "#5e5ce6",
    accentBg: "rgba(94,92,230,0.08)",
    label: "PRESETS",
    title: "One click to reload your frequent transfers.",
    body: "Save any source/destination/options combo by name. Load it instantly next session.",
    mockup: <PresetMockup />,
  },
  {
    col: "col-span-2",
    accent: "var(--red)",
    accentBg: "rgba(255,69,58,0.06)",
    label: "BUILT-IN SAFEGUARDS",
    title: "Destructive actions require confirmation.",
    body: "Sync mode, cross-drive moves, and merge-into-root combinations all trigger clear warning modals before anything is deleted. You're always in control.",
    mockup: <SyncMockup />,
  },
];

export default function Features() {
  const ref = useRef(null);
  const inView = useInView(ref, { once: true, margin: "-60px" });

  return (
    <section ref={ref} style={{ padding: "100px 24px" }}>
      <div style={{ maxWidth: 960, margin: "0 auto" }}>
        {/* Label */}
        <motion.p
          initial={{ opacity: 0, y: 12 }}
          animate={inView ? { opacity: 1, y: 0 } : {}}
          transition={{ duration: 0.4 }}
          style={{
            textAlign: "center",
            fontSize: 11,
            fontWeight: 700,
            letterSpacing: "0.08em",
            textTransform: "uppercase",
            color: "var(--text-tertiary)",
            marginBottom: 14,
          }}
        >
          Features
        </motion.p>

        {/* Heading */}
        <motion.h2
          initial={{ opacity: 0, y: 16 }}
          animate={inView ? { opacity: 1, y: 0 } : {}}
          transition={{ duration: 0.5, delay: 0.06 }}
          style={{
            textAlign: "center",
            fontSize: "clamp(28px, 5vw, 44px)",
            fontWeight: 700,
            letterSpacing: "-0.03em",
            color: "var(--text-primary)",
            margin: "0 0 56px",
            lineHeight: 1.1,
          }}
        >
          Everything a power user actually needs.
        </motion.h2>

        {/* Bento grid */}
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(2, 1fr)",
            gap: 14,
          }}
        >
          {cards.map((card, i) => (
            <motion.div
              key={card.label}
              initial={{ opacity: 0, y: 28 }}
              animate={inView ? { opacity: 1, y: 0 } : {}}
              transition={{
                duration: 0.55,
                delay: 0.08 + i * 0.06,
                ease: "easeOut",
              }}
              whileHover={{
                borderColor: "rgba(255,255,255,0.14)",
                y: -2,
              }}
              style={{
                gridColumn: card.col,
                padding: "24px",
                borderRadius: 16,
                border: "1px solid var(--glass-border)",
                background: card.accentBg,
                backdropFilter: "blur(24px)",
                transition: "border-color 0.2s, transform 0.2s",
                display: "flex",
                flexDirection: "column",
              }}
            >
              {/* Label */}
              <span
                style={{
                  fontSize: 9,
                  fontWeight: 700,
                  letterSpacing: "0.08em",
                  textTransform: "uppercase",
                  color: card.accent,
                  marginBottom: 10,
                }}
              >
                {card.label}
              </span>

              {/* Title */}
              <h3
                style={{
                  fontSize: 18,
                  fontWeight: 600,
                  letterSpacing: "-0.02em",
                  color: "var(--text-primary)",
                  margin: "0 0 8px",
                  lineHeight: 1.25,
                }}
              >
                {card.title}
              </h3>

              {/* Body */}
              <p
                style={{
                  fontSize: 13,
                  color: "var(--text-secondary)",
                  lineHeight: 1.6,
                  margin: "0 0 16px",
                }}
              >
                {card.body}
              </p>

              {/* Mockup */}
              <div style={{ marginTop: "auto" }}>{card.mockup}</div>
            </motion.div>
          ))}
        </div>
      </div>

      {/* Spin keyframe for progress spinner */}
      <style>{`
        @keyframes spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
      `}</style>
    </section>
  );
}
