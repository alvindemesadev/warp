"use client";

import { useEffect, useState } from "react";
import { motion } from "framer-motion";

export default function Nav() {
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 16);
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <motion.header
      initial={{ opacity: 0, y: -12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4, ease: "easeOut" }}
      className="fixed top-0 left-0 right-0 z-50 flex items-center justify-between px-6 py-3"
      style={{
        background: scrolled ? "rgba(0,0,0,0.72)" : "transparent",
        borderBottom: scrolled ? "1px solid rgba(255,255,255,0.06)" : "1px solid transparent",
        backdropFilter: scrolled ? "blur(48px) saturate(180%)" : "none",
        transition: "background 0.3s, border-color 0.3s, backdrop-filter 0.3s",
      }}
    >
      {/* Logo */}
      <a
        href="#"
        className="flex items-center gap-2 no-underline"
        style={{ color: "var(--text-primary)" }}
      >
        {/* Bolt icon matching app aesthetic */}
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none">
          <path
            d="M13 2L4.5 13.5H11.5L11 22L19.5 10.5H12.5L13 2Z"
            fill="var(--accent)"
            stroke="var(--accent)"
            strokeWidth="1.5"
            strokeLinejoin="round"
          />
        </svg>
        <span style={{ fontWeight: 700, fontSize: 17, letterSpacing: "-0.03em" }}>
          Warp
        </span>
      </a>

      {/* Right side */}
      <div className="flex items-center gap-3">
        <a
          href="https://github.com/alvindemesadev/warp"
          target="_blank"
          rel="noopener noreferrer"
          className="hidden sm:flex items-center gap-1.5 transition-colors"
          style={{ fontSize: 13, fontWeight: 500, color: "var(--text-secondary)" }}
          onMouseEnter={(e) =>
            ((e.currentTarget as HTMLElement).style.color = "var(--text-primary)")
          }
          onMouseLeave={(e) =>
            ((e.currentTarget as HTMLElement).style.color = "var(--text-secondary)")
          }
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" />
          </svg>
          GitHub
        </a>

        <a
          href="#download"
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 6,
            padding: "7px 16px",
            borderRadius: 10,
            background: "var(--accent)",
            color: "#fff",
            fontSize: 13,
            fontWeight: 600,
            textDecoration: "none",
            boxShadow: "0 2px 16px rgba(10,132,255,0.35)",
            transition: "background 0.15s, box-shadow 0.15s",
            letterSpacing: "-0.01em",
          }}
          onMouseEnter={(e) => {
            (e.currentTarget as HTMLElement).style.background = "var(--accent-hover)";
            (e.currentTarget as HTMLElement).style.boxShadow = "0 4px 24px rgba(10,132,255,0.5)";
          }}
          onMouseLeave={(e) => {
            (e.currentTarget as HTMLElement).style.background = "var(--accent)";
            (e.currentTarget as HTMLElement).style.boxShadow = "0 2px 16px rgba(10,132,255,0.35)";
          }}
        >
          Download Free
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path d="M6 2v6M3 6l3 3 3-3" stroke="white" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"/>
          </svg>
        </a>
      </div>
    </motion.header>
  );
}
