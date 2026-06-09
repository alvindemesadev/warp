"use client";

import { motion, useInView } from "framer-motion";
import { useRef } from "react";

const steps = [
  {
    number: "01",
    accentBg: "rgba(10,132,255,0.10)",
    icon: (
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
        <path d="M12 3v12M8 11l4 4 4-4" stroke="var(--accent)" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"/>
        <path d="M4 17v2a2 2 0 002 2h12a2 2 0 002-2v-2" stroke="var(--accent)" strokeWidth="1.8" strokeLinecap="round"/>
      </svg>
    ),
    title: "Drop",
    body: "Drag your source and destination folders directly onto the window. Or use Ctrl+O to open the folder browser dialog.",
  },
  {
    number: "02",
    accentBg: "rgba(10,132,255,0.10)",
    icon: (
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
        <rect x="3" y="8" width="18" height="10" rx="3" stroke="var(--accent)" strokeWidth="1.8"/>
        <path d="M7 8V6a5 5 0 0110 0v2" stroke="var(--accent)" strokeWidth="1.8" strokeLinecap="round"/>
        <circle cx="12" cy="13" r="1.5" fill="var(--accent)"/>
      </svg>
    ),
    title: "Configure",
    body: "Choose Copy, Move, or Sync. Set a speed limit, conflict rule, and verify option. Save it as a preset to skip this next time.",
  },
  {
    number: "03",
    accentBg: "rgba(48,209,88,0.10)",
    icon: (
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
        <path d="M5 12l5 5L20 7" stroke="var(--green)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
      </svg>
    ),
    title: "Transfer",
    body: "Press Enter. Watch files move in real time with per-file names, live speed, and ETA. Get a system notification when done.",
  },
];

export default function HowItWorks() {
  const ref = useRef(null);
  const inView = useInView(ref, { once: true, margin: "-60px" });

  return (
    <section
      ref={ref}
      style={{
        padding: "100px 24px",
        background: "var(--surface-1)",
        borderTop: "1px solid var(--glass-border)",
        borderBottom: "1px solid var(--glass-border)",
        position: "relative",
        overflow: "hidden",
      }}
    >
      <div aria-hidden style={{
        position: "absolute", top: "50%", left: "50%",
        transform: "translate(-50%,-50%)", width: 600, height: 300,
        background: "radial-gradient(ellipse, rgba(10,132,255,0.07) 0%, transparent 70%)",
        pointerEvents: "none",
      }} />

      <div style={{ maxWidth: 960, margin: "0 auto", position: "relative" }}>
        <motion.p
          initial={{ opacity: 0, y: 12 }}
          animate={inView ? { opacity: 1, y: 0 } : {}}
          transition={{ duration: 0.4 }}
          style={{
            textAlign: "center", fontSize: 11, fontWeight: 700,
            letterSpacing: "0.08em", textTransform: "uppercase",
            color: "var(--text-tertiary)", marginBottom: 14,
          }}
        >
          How it works
        </motion.p>

        <motion.h2
          initial={{ opacity: 0, y: 16 }}
          animate={inView ? { opacity: 1, y: 0 } : {}}
          transition={{ duration: 0.5, delay: 0.06 }}
          style={{
            textAlign: "center", fontSize: "clamp(28px, 5vw, 44px)",
            fontWeight: 700, letterSpacing: "-0.03em",
            color: "var(--text-primary)", margin: "0 0 64px", lineHeight: 1.1,
          }}
        >
          From drop to done in three steps.
        </motion.h2>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: 0, position: "relative" }}>
          {steps.map((step, i) => (
            <motion.div
              key={step.number}
              initial={{ opacity: 0, y: 24 }}
              animate={inView ? { opacity: 1, y: 0 } : {}}
              transition={{ duration: 0.5, delay: 0.1 + i * 0.1, ease: "easeOut" }}
              style={{
                padding: "32px 28px", position: "relative",
                borderRight: i < steps.length - 1 ? "1px solid var(--glass-border)" : "none",
              }}
            >
              <span style={{ fontSize: 11, fontWeight: 700, letterSpacing: "0.08em", color: "var(--text-tertiary)", display: "block", marginBottom: 16 }}>
                {step.number}
              </span>
              <div style={{
                width: 48, height: 48, borderRadius: 14,
                background: step.accentBg,
                display: "flex", alignItems: "center", justifyContent: "center", marginBottom: 20,
              }}>
                {step.icon}
              </div>
              <h3 style={{ fontSize: 22, fontWeight: 700, letterSpacing: "-0.03em", color: "var(--text-primary)", margin: "0 0 10px" }}>
                {step.title}
              </h3>
              <p style={{ fontSize: 14, color: "var(--text-secondary)", lineHeight: 1.65, margin: 0 }}>
                {step.body}
              </p>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
}
