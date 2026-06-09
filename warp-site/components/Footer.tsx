"use client";

export default function Footer() {
  return (
    <footer
      style={{
        borderTop: "1px solid var(--glass-border)",
        padding: "32px 24px",
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        justifyContent: "space-between",
        gap: 16,
        maxWidth: 960,
        margin: "0 auto",
      }}
    >
      {/* Left — brand */}
      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
          <path
            d="M13 2L4.5 13.5H11.5L11 22L19.5 10.5H12.5L13 2Z"
            fill="var(--accent)"
            stroke="var(--accent)"
            strokeWidth="1.5"
            strokeLinejoin="round"
          />
        </svg>
        <span style={{ fontSize: 14, fontWeight: 600, color: "var(--text-secondary)", letterSpacing: "-0.02em" }}>
          Warp
        </span>
        <span style={{ fontSize: 12, color: "var(--text-tertiary)" }}>v1.0.0</span>
        <span style={{ fontSize: 12, color: "var(--text-tertiary)" }}>·</span>
        <span style={{ fontSize: 12, color: "var(--text-tertiary)" }}>MIT License</span>
        <span style={{ fontSize: 12, color: "var(--text-tertiary)" }}>·</span>
        <span style={{ fontSize: 12, color: "var(--text-tertiary)" }}>Built by Alvin</span>
      </div>

      {/* Right — links */}
      <div style={{ display: "flex", alignItems: "center", gap: 20 }}>
        {[
          {
            label: "GitHub",
            href: "https://github.com/alvindemesadev/warp",
            icon: (
              <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                <path d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" />
              </svg>
            ),
          },
          {
            label: "Releases",
            href: "https://github.com/alvindemesadev/warp/releases",
            icon: (
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                <path d="M12 2v9M9 8l3 3 3-3" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"/>
                <path d="M4 14v5a1 1 0 001 1h14a1 1 0 001-1v-5" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round"/>
              </svg>
            ),
          },
        ].map((link) => (
          <a
            key={link.label}
            href={link.href}
            target="_blank"
            rel="noopener noreferrer"
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 5,
              fontSize: 12,
              color: "var(--text-tertiary)",
              textDecoration: "none",
              transition: "color 0.15s",
            }}
            onMouseEnter={(e) =>
              ((e.currentTarget as HTMLElement).style.color = "var(--text-secondary)")
            }
            onMouseLeave={(e) =>
              ((e.currentTarget as HTMLElement).style.color = "var(--text-tertiary)")
            }
          >
            {link.icon}
            {link.label}
          </a>
        ))}
      </div>
    </footer>
  );
}
