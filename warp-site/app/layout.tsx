import type { Metadata } from "next";
import { Inter } from "next/font/google";
import "./globals.css";

const inter = Inter({
  subsets: ["latin"],
  variable: "--font-inter",
  display: "swap",
});

export const metadata: Metadata = {
  title: "Warp — High-speed File Transfer for Windows",
  description:
    "Warp wraps Windows' built-in robocopy in a clean, modern interface. Real-time progress, live speed, transfer queue, presets, and zero command line. Free forever.",
  keywords: ["file transfer", "windows", "robocopy", "copy files", "move files", "sync folders"],
  authors: [{ name: "Alvin" }],
  openGraph: {
    title: "Warp — High-speed File Transfer for Windows",
    description: "Move files at the speed of thought. Free Windows desktop app.",
    images: ["/screenshot.png"],
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "Warp — High-speed File Transfer for Windows",
    description: "Move files at the speed of thought. Free Windows desktop app.",
    images: ["/screenshot.png"],
  },
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className={inter.variable}>
      <body className="noise">{children}</body>
    </html>
  );
}
