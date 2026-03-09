import type { NextConfig } from "next";
import path from "node:path";

const nextConfig: NextConfig = {
  outputFileTracingRoot: path.resolve(import.meta.dirname, "../../"),
};

export default nextConfig;
