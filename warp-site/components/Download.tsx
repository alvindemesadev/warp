"use client";

import { motion, useInView } from "framer-motion";
import { useRef } from "react";

export default function Download() {
  const ref = useRef(null);
  const inView = useInView(ref, { once: true, margin: "-60px" });

  return (
    <section
      id="download"
      ref={ref}
      style={{ padding: "120px 24px", position: "relative", overflow: "hidden" }}
    >
      {/* Large blue glow */}
      <div
        aria-hidden
        style={{
          position: "absolute",
          top: "50%",
          left: "50%",
          transform: "translate(-50%,-50%)",
          width: 800,
          height: 400,
          background: "radial-gradient(ellipse, rgba(10,132,255,0.14) 0%, transparent 65%)",
          pointerEvents: "none",
        }}
      />

      <div
        style={{
          maxWidth: 600,
          margin: "0 auto",
          textAlign: "center",
          position: "relative",
        }}
      >
        {/* Label */}
        <motion.p
          initial={{ opacity: 0, y: 12 }}
          animate={inView ? { opacity: 1, y: 0 } : {}}
          transition={{ duration: 0.4 }}
          style={{
            fontSize: 11,
            fontWeight: 700,
            letterSpacing: "0.08em",
            textTransform: "uppercase",
            color: "var(--text-tertiary)",
            marginBottom: 14,
          }}
        >
          Download
        </motion.p>

        {/* Heading */}
        <motion.h2
          initial={{ opacity: 0, y: 16 }}
          animate={inView ? { opacity: 1, y: 0 } : {}}
          transition={{ duration: 0.5, delay: 0.06 }}
          style={{
            fontSize: "clamp(32px, 6vw, 52px)",
            fontWeight: 700,
            letterSpacing: "-0.04em",
            color: "var(--text-primary)",
            margin: "0 0 16px",
            lineHeight: 1.1,
          }}
        >
          Ready to move faster?
        </motion.h2>

        {/* Subhead */}
        <motion.p
          initial={{ opacity: 0, y: 12 }}
          animate={inView ? { opacity: 1, y: 0 } : {}}
          transition={{ duration: 0.5, delay: 0.1 }}
          style={{
            fontSize: 17,
            color: "var(--text-secondary)",
            lineHeight: 1.6,
            margin: "0 0 44px",
          }}
        >
          Free forever. No account. No telemetry.
          <br />
          Just a fast file transfer tool that gets out of your way.
        </motion.p>

        {/* Download buttons */}
        <motion.div
          initial={{ opacity: 0, y: 16 }}
          animate={inView ? { opacity: 1, y: 0 } : {}}
          transition={{ duration: 0.5, delay: 0.14 }}
          style={{
            display: "flex",
            flexWrap: "wrap",
            gap: 12,
            justifyContent: "center",
            marginBottom: 28,
          }}
        >
          {/* Primary — NSIS .exe */}
          <a
            href="https://github.com/alvindemesadev/warp/releases/latest"
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 8,
              padding: "14px 28px",
              borderRadius: 12,
              background: "var(--accent)",
              color: "#fff",
              fontSize: 15,
              fontWeight: 600,
              textDecoration: "none",
              letterSpacing: "-0.01em",
              boxShadow: "0 4px 28px rgba(10,132,255,0.45)",
              transition: "background 0.15s, box-shadow 0.15s, transform 0.1s",
            }}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLElement).style.background = "var(--accent-hover)";
              (e.currentTarget as HTMLElement).style.boxShadow = "0 6px 36px rgba(10,132,255,0.6)";
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLElement).style.background = "var(--accent)";
              (e.currentTarget as HTMLElement).style.boxShadow = "0 4px 28px rgba(10,132,255,0.45)";
            }}
            onMouseDown={(e) => {
              (e.currentTarget as HTMLElement).style.transform = "scale(0.97)";
            }}
            onMouseUp={(e) => {
              (e.currentTarget as HTMLElement).style.transform = "scale(1)";
            }}
          >
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M8 2v9M5 8l3 3 3-3" stroke="white" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"/>
              <path d="M2 13h12" stroke="white" strokeWidth="1.8" strokeLinecap="round"/>
            </svg>
            Download .exe
            <span
              style={{
                fontSize: 11,
                fontWeight: 500,
                opacity: 0.75,
                background: "rgba(255,255,255,0.15)",
                borderRadius: 5,
                padding: "2px 6px",
              }}
            >
              3.6 MB
            </span>
          </a>

          {/* Secondary — MSI */}
          <a
            href="https://github.com/alvindemesadev/warp/releases/latest"
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 8,
              padding: "14px 24px",
              borderRadius: 12,
              border: "1px solid var(--glass-border)",
              background: "rgba(255,255,255,0.04)",
              color: "var(--text-secondary)",
              fontSize: 15,
              fontWeight: 500,
              textDecoration: "none",
              letterSpacing: "-0.01em",
              transition: "background 0.15s, color 0.15s",
            }}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLElement).style.background = "rgba(255,255,255,0.08)";
              (e.currentTarget as HTMLElement).style.color = "var(--text-primary)";
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLElement).style.background = "rgba(255,255,255,0.04)";
              (e.currentTarget as HTMLElement).style.color = "var(--text-secondary)";
            }}
          >
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M8 2v9M5 8l3 3 3-3" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"/>
              <path d="M2 13h12" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round"/>
            </svg>
            Download .msi
            <span
              style={{
                fontSize: 11,
                opacity: 0.6,
              }}
            >
              4.8 MB
            </span>
          </a>
        </motion.div>

        {/* Requirements */}
        <motion.div
          initial={{ opacity: 0 }}
          animate={inView ? { opacity: 1 } : {}}
          transition={{ duration: 0.5, delay: 0.2 }}
        >
          <p
            style={{
              fontSize: 12,
              color: "var(--text-tertiary)",
              margin: "0 0 20px",
              letterSpacing: "0.01em",
            }}
          >
            Windows 10 / 11 (64-bit) · WebView2 (embedded in installer) · Robocopy (built into Windows)
          </p>

          {/* SmartScreen notice */}
          <div
            style={{
              display: "inline-flex",
              alignItems: "flex-start",
              gap: 10,
              padding: "12px 16px",
              borderRadius: 10,
              border: "1px solid rgba(255,159,10,0.25)",
              background: "rgba(255,159,10,0.06)",
              textAlign: "left",
              maxWidth: 480,
            }}
          >
            <svg width="16" height="16" viewBox="0 0 20 20" fill="none" style={{ flexShrink: 0, marginTop: 1 }}>
              <path d="M10 3l7.5 13H2.5L10 3z" stroke="var(--orange)" strokeWidth="1.5" fill="none" strokeLinejoin="round"/>
              <path d="M10 8v4m0 2.5v.5" stroke="var(--orange)" strokeWidth="1.5" strokeLinecap="round"/>
            </svg>
            <p style={{ fontSize: 12, color: "rgba(255,159,10,0.9)", margin: 0, lineHeight: 1.6 }}>
              Windows SmartScreen may show &quot;Windows protected your PC&quot; — the installer is{" "}
              unsigned (no paid certificate). Click{" "}
              <strong>More info → Run anyway</strong> to install.
            </p>
          </div>
        </motion.div>
      </div>
    </section>
  );
}
