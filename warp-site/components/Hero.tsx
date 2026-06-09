"use client";

import { motion } from "framer-motion";
import Image from "next/image";

const fadeUp = {
  hidden: { opacity: 0, y: 24 },
  show: (i: number) => ({
    opacity: 1,
    y: 0,
    transition: { duration: 0.55, ease: "easeOut" as const, delay: i * 0.08 },
  }),
};

export default function Hero() {
  return (
    <section
      className="relative flex flex-col items-center text-center overflow-hidden"
      style={{ paddingTop: "140px", paddingBottom: "100px", minHeight: "100vh" }}
    >
      {/* Blue radial glow — identical to the app background */}
      <div
        aria-hidden
        style={{
          position: "absolute",
          top: -80,
          left: "50%",
          transform: "translateX(-50%)",
          width: 700,
          height: 400,
          background: "radial-gradient(ellipse, rgba(10,132,255,0.18) 0%, transparent 70%)",
          borderRadius: "50%",
          filter: "blur(1px)",
          pointerEvents: "none",
        }}
      />

      <div className="relative z-10 flex flex-col items-center px-6" style={{ maxWidth: 720 }}>
        {/* Badge */}
        <motion.div
          custom={0}
          variants={fadeUp}
          initial="hidden"
          animate="show"
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 6,
            padding: "4px 12px",
            borderRadius: 100,
            border: "1px solid rgba(10,132,255,0.3)",
            background: "rgba(10,132,255,0.08)",
            marginBottom: 28,
          }}
        >
          <span
            style={{
              width: 6,
              height: 6,
              borderRadius: "50%",
              background: "var(--accent)",
              display: "inline-block",
            }}
          />
          <span
            style={{
              fontSize: 11,
              fontWeight: 700,
              color: "var(--accent)",
              letterSpacing: "0.06em",
              textTransform: "uppercase",
            }}
          >
            Free · Open Source · v1.0.0
          </span>
        </motion.div>

        {/* Headline */}
        <motion.h1
          custom={1}
          variants={fadeUp}
          initial="hidden"
          animate="show"
          style={{
            fontSize: "clamp(44px, 8vw, 80px)",
            fontWeight: 700,
            letterSpacing: "-0.04em",
            lineHeight: 1.0,
            color: "var(--text-primary)",
            margin: "0 0 20px",
          }}
        >
          Move files at the{" "}
          <span
            style={{
              background: "linear-gradient(135deg, var(--accent), #5e5ce6)",
              WebkitBackgroundClip: "text",
              WebkitTextFillColor: "transparent",
              backgroundClip: "text",
            }}
          >
            speed of thought.
          </span>
        </motion.h1>

        {/* Subheadline */}
        <motion.p
          custom={2}
          variants={fadeUp}
          initial="hidden"
          animate="show"
          style={{
            fontSize: "clamp(16px, 2.5vw, 19px)",
            fontWeight: 400,
            color: "var(--text-secondary)",
            lineHeight: 1.6,
            maxWidth: 540,
            margin: "0 0 40px",
          }}
        >
          Warp wraps Windows&apos; built-in robocopy in a clean, modern interface —
          real-time progress, live speed, transfer queue, and{" "}
          <span style={{ color: "var(--text-primary)", fontWeight: 500 }}>
            zero command line.
          </span>
        </motion.p>

        {/* CTAs */}
        <motion.div
          custom={3}
          variants={fadeUp}
          initial="hidden"
          animate="show"
          className="flex flex-wrap items-center justify-center gap-3"
          style={{ marginBottom: 20 }}
        >
          <a
            href="#download"
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 8,
              padding: "12px 24px",
              borderRadius: 12,
              background: "var(--accent)",
              color: "#fff",
              fontSize: 15,
              fontWeight: 600,
              textDecoration: "none",
              letterSpacing: "-0.01em",
              boxShadow: "0 4px 24px rgba(10,132,255,0.4)",
              transition: "background 0.15s, transform 0.1s, box-shadow 0.15s",
            }}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLElement).style.background = "var(--accent-hover)";
              (e.currentTarget as HTMLElement).style.boxShadow = "0 6px 32px rgba(10,132,255,0.55)";
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLElement).style.background = "var(--accent)";
              (e.currentTarget as HTMLElement).style.boxShadow = "0 4px 24px rgba(10,132,255,0.4)";
            }}
            onMouseDown={(e) => {
              (e.currentTarget as HTMLElement).style.transform = "scale(0.97)";
            }}
            onMouseUp={(e) => {
              (e.currentTarget as HTMLElement).style.transform = "scale(1)";
            }}
          >
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
              <path d="M7 2v8M4 7l3 3 3-3" stroke="white" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"/>
              <path d="M2 12h10" stroke="white" strokeWidth="1.8" strokeLinecap="round"/>
            </svg>
            Download for Windows — Free
          </a>

          <a
            href="https://github.com/alvindemesadev/warp"
            target="_blank"
            rel="noopener noreferrer"
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 8,
              padding: "12px 22px",
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
            <svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" />
            </svg>
            View on GitHub
            <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
              <path d="M2 8L8 2M8 2H4M8 2v4" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"/>
            </svg>
          </a>
        </motion.div>

        {/* Trust line */}
        <motion.p
          custom={4}
          variants={fadeUp}
          initial="hidden"
          animate="show"
          style={{
            fontSize: 12,
            color: "var(--text-tertiary)",
            letterSpacing: "0.01em",
            marginBottom: 64,
          }}
        >
          Windows 10 / 11 (64-bit) · No account · No telemetry · MIT licensed
        </motion.p>

        {/* App screenshot */}
        <motion.div
          custom={5}
          variants={fadeUp}
          initial="hidden"
          animate="show"
          style={{
            position: "relative",
            borderRadius: 18,
            border: "1px solid rgba(255,255,255,0.10)",
            boxShadow:
              "0 0 0 1px rgba(255,255,255,0.05), 0 32px 80px rgba(0,0,0,0.7), 0 0 80px rgba(10,132,255,0.08)",
            overflow: "hidden",
            width: "100%",
            maxWidth: 660,
          }}
        >
          {/* Subtle glow under the screenshot */}
          <div
            aria-hidden
            style={{
              position: "absolute",
              bottom: -40,
              left: "50%",
              transform: "translateX(-50%)",
              width: "80%",
              height: 80,
              background: "radial-gradient(ellipse, rgba(10,132,255,0.2), transparent 70%)",
              filter: "blur(20px)",
              pointerEvents: "none",
            }}
          />
          <Image
            src="/screenshot.png"
            alt="Warp app interface showing file transfer in progress"
            width={660}
            height={480}
            priority
            style={{ display: "block", width: "100%", height: "auto" }}
          />
        </motion.div>
      </div>
    </section>
  );
}
