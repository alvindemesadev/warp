"use client";

import { motion, useInView } from "framer-motion";
import { useRef } from "react";

const problems = [
  {
    icon: (
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
        <circle cx="12" cy="12" r="9" stroke="currentColor" strokeWidth="1.5"/>
        <path d="M12 7v5l3 3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
      </svg>
    ),
    title: "No real progress",
    body: "Windows Explorer shows a spinner and a vague estimate that's wrong 80% of the time. You have no idea how fast files are actually moving.",
  },
  {
    icon: (
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
        <rect x="3" y="3" width="7" height="7" rx="1.5" stroke="currentColor" strokeWidth="1.5"/>
        <rect x="14" y="3" width="7" height="7" rx="1.5" stroke="currentColor" strokeWidth="1.5"/>
        <rect x="3" y="14" width="7" height="7" rx="1.5" stroke="currentColor" strokeWidth="1.5"/>
        <path d="M17.5 14v7M14 17.5h7" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
      </svg>
    ),
    title: "One job at a time",
    body: "You can't queue multiple transfers. You sit and watch, then manually start the next one. Every. Single. Time.",
  },
  {
    icon: (
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
        <path d="M12 2L2 7l10 5 10-5-10-5z" stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round"/>
        <path d="M2 17l10 5 10-5M2 12l10 5 10-5" stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round"/>
      </svg>
    ),
    title: "Zero control",
    body: "Can't throttle speed. Can't verify the copy finished correctly. Can't save your frequent transfers as a preset.",
  },
];

export default function Problem() {
  const ref = useRef(null);
  const inView = useInView(ref, { once: true, margin: "-80px" });

  return (
    <section ref={ref} style={{ padding: "100px 24px", position: "relative" }}>
      <div style={{ maxWidth: 960, margin: "0 auto" }}>

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
          Why Warp
        </motion.p>

        <motion.h2
          initial={{ opacity: 0, y: 16 }}
          animate={inView ? { opacity: 1, y: 0 } : {}}
          transition={{ duration: 0.5, delay: 0.06 }}
          style={{
            textAlign: "center",
            fontSize: "clamp(28px, 5vw, 44px)",
            fontWeight: 700, letterSpacing: "-0.03em",
            color: "var(--text-primary)", margin: "0 0 56px", lineHeight: 1.1,
          }}
        >
          Windows file transfers are stuck in 2003.
        </motion.h2>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))", gap: 16 }}>
          {problems.map((p, i) => (
            <motion.div
              key={p.title}
              initial={{ opacity: 0, y: 24 }}
              animate={inView ? { opacity: 1, y: 0 } : {}}
              transition={{ duration: 0.5, delay: 0.1 + i * 0.08, ease: "easeOut" }}
              whileHover={{ borderColor: "rgba(255,255,255,0.15)" }}
              style={{
                padding: "24px", borderRadius: 16,
                border: "1px solid var(--glass-border)", background: "var(--surface-1)",
                transition: "border-color 0.2s",
              }}
            >
              <div style={{
                width: 40, height: 40, borderRadius: 10,
                background: "rgba(255,69,58,0.12)", color: "var(--red)",
                display: "flex", alignItems: "center", justifyContent: "center",
                marginBottom: 16, flexShrink: 0,
              }}>
                {p.icon}
              </div>
              <h3 style={{ fontSize: 16, fontWeight: 600, color: "var(--text-primary)", margin: "0 0 8px", letterSpacing: "-0.02em" }}>
                {p.title}
              </h3>
              <p style={{ fontSize: 14, color: "var(--text-secondary)", lineHeight: 1.6, margin: 0 }}>
                {p.body}
              </p>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
}
