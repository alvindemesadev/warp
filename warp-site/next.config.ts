import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  output: "export",   // static export → GitHub Pages / Vercel
  images: {
    unoptimized: true, // required for static export
  },
};

export default nextConfig;
