/** @type {import("next").NextConfig} */
let nextConfig = {
  images: {
    unoptimized: true,
  },
  reactStrictMode: true,
};

const isProduction = process.env.NODE_ENV === "production";
if (isProduction) {
  const assetPrefix = "/scarb";
  nextConfig = {
    ...nextConfig,
    assetPrefix,
    basePath: assetPrefix,
  };
}

const withNextra = require("nextra")({
  theme: "nextra-theme-docs",
  themeConfig: "./theme.config.tsx",
});

module.exports = withNextra(nextConfig);
