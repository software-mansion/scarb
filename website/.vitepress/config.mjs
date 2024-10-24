import { defineConfig } from "vitepress";
import { withMermaid } from "vitepress-plugin-mermaid";
import * as syntaxes from "./syntaxes.mjs";

const base = "/scarb/";
const absoluteBase = `https://docs.swmansion.com${base}`;
const lang = "en-US";

// https://vitepress.dev/reference/site-config
const sidebar = {
  "/docs": [
    {
      text: "Getting started",
      items: [p("Introduction", "/docs"), p("Cheat sheet", "/docs/cheatsheet")],
    },
    {
      text: "Guides",
      items: [
        p("Creating a new package", "/docs/guides/creating-a-new-package"),
        p(
          "Working on existing package",
          "/docs/guides/working-on-an-existing-package",
        ),
        p("Dependencies", "/docs/guides/dependencies"),
        p("Formatting", "/docs/guides/formatting"),
        p("Defining custom profiles", "/docs/guides/defining-custom-profiles"),
        p("Defining custom scripts", "/docs/guides/defining-custom-scripts"),
        p("Using Scarb in CI", "/docs/guides/using-scarb-in-ci"),
      ],
    },
    {
      text: "Core extensions",
      items: [
        p("Testing", "/docs/extensions/testing"),
        p("Cairo runner", "/docs/extensions/cairo-run"),
        {
          text: "Starknet",
          items: [
            p("Contract target", "/docs/extensions/starknet/contract-target"),
            p("Starknet package", "/docs/extensions/starknet/starknet-package"),
          ],
        },
        p("Language server", "/docs/extensions/cairo-language-server"),
        p("Expand", "/docs/extensions/expand"),
        p(
          "Generating documentation",
          "/docs/extensions/documentation-generation",
        ),
      ],
    },
    {
      text: "Reference",
      items: [
        p("Compilation model", "/docs/reference/compilation-model"),
        p("Conditional compilation", "/docs/reference/conditional-compilation"),
        p("Procedural Macros", "/docs/reference/procedural-macro"),
        p("Global directories", "/docs/reference/global-directories"),
        p("Manifest", "/docs/reference/manifest"),
        p("Lockfile", "/docs/reference/lockfile"),
        p("Workspaces", "/docs/reference/workspaces"),
        p("Profiles", "/docs/reference/profiles"),
        p("Scripts", "/docs/reference/scripts"),
        p("Specifying dependencies", "/docs/reference/specifying-dependencies"),
        p("Targets", "/docs/reference/targets"),
        p(
          "Corelib documentation",
          "https://docs.swmansion.com/scarb/corelib/index.html",
        ),
      ],
    },
    {
      text: "Writing extensions",
      items: [
        p("JSON output", "/docs/writing-extensions/json-output"),
        p("Scarb metadata", "/docs/writing-extensions/scarb-metadata"),
        p("Subcommands", "/docs/writing-extensions/subcommands"),
        p("Scarb crate", "/docs/writing-extensions/scarb-crate"),
      ],
    },
    {
      text: "Registries",
      items: [
        p("Overview", "/docs/registries/overview"),
        p("Publishing", "/docs/registries/publishing"),
        p("Package tarball", "/docs/registries/package-tarball"),
        p("Custom registry", "/docs/registries/custom-registry"),
      ],
    },
    {
      text: "Appendices",
      items: [
        p(
          "Examples",
          "https://github.com/software-mansion/scarb/tree/main/examples",
        ),
        p("Scarb vs Cargo", "/docs/scarb-vs-cargo"),
        p("Troubleshooting", "/docs/troubleshooting"),
      ],
    },
  ],
};

const telegramIcon = `
  <svg role="img" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
    <title>Telegram</title>
    <path d="M12 0C5.37058 0 0 5.37135 0 12C0 18.6286 5.37135 24 12 24C18.6294 24 24 18.6286 24 12C24 5.37135 18.6286 0 12 0ZM17.8939 8.22116L15.9244 17.5022C15.7788 18.1603 15.3871 18.3197 14.8405 18.0101L11.8405 15.799L10.3935 17.1925C10.2341 17.352 10.0986 17.4875 9.7889 17.4875L10.0018 14.4341L15.5613 9.4111C15.8036 9.19819 15.5079 9.07742 15.1881 9.29032L8.31716 13.6157L5.35587 12.6914C4.71252 12.4885 4.69781 12.048 5.49135 11.7383L17.0609 7.27665C17.5982 7.0831 18.0674 7.40748 17.8932 8.22039L17.8939 8.22116Z"/>
  </svg>
`;

export default withMermaid(
  defineConfig({
    title: "Scarb",
    description:
      "Scarb is a build toolchain and package manager for Cairo and Starknet ecosystems.",
    lang,
    base,

    head: [
      ["meta", { httpEquiv: "Content-Language", content: lang }],
      ["link", { rel: "manifest", href: `${base}manifest.json` }],
      ["link", { rel: "icon", href: `${base}favicon.ico`, sizes: "any" }],
      [
        "link",
        { rel: "icon", href: `${base}favicon.svg`, type: "image/svg+xml" },
      ],
      [
        "link",
        { rel: "apple-touch-icon", href: `${base}apple-touch-icon.png` },
      ],
      ["meta", { name: "apple-mobile-web-app-title", content: "Scarb" }],
      ["meta", { name: "twitter:card", content: "summary_large_image" }],
      ["meta", { name: "twitter:site", content: "@swmansionxyz" }],
      ["meta", { name: "twitter:creator", content: "@jajakobyly" }],
      [
        "meta",
        {
          property: "og:title",
          content: "Scarb, the Cairo and StarkNet development toolchain",
        },
      ],
      [
        "meta",
        {
          property: "og:description",
          content:
            "Scarb is a build toolchain and package manager for Cairo and Starknet ecosystems.",
        },
      ],
      ["meta", { property: "og:type", content: "website" }],
      [
        "meta",
        { property: "og:image", content: `${absoluteBase}og-image.png` },
      ],
      [
        "meta",
        {
          property: "og:image:alt",
          content:
            "Scarb is a build toolchain and package manager for Cairo and Starknet ecosystems.",
        },
      ],
      ["meta", { property: "og:image:type", content: "image/png" }],
      ["meta", { property: "og:image:width", content: "1280" }],
      ["meta", { property: "og:image:height", content: "640" }],
      [
        "script",
        {
          "data-goatcounter": "https://gc-scarb.swmtest.xyz/count",
          async: true,
          src: `${base}count.js`,
        },
      ],
    ],

    lastUpdated: true,

    themeConfig: {
      logo: {
        light: "/logo-light.svg",
        dark: "/logo-dark.svg",
        alt: "Scarb",
      },
      siteTitle: false,

      nav: [
        { text: "Download", link: "/download" },
        { text: "Documentation", link: "/docs" },
      ],

      sidebar,

      socialLinks: [
        { icon: "github", link: "https://github.com/software-mansion/scarb" },
        { icon: "twitter", link: "https://twitter.com/swmansionxyz" },
        {
          icon: {
            svg: telegramIcon,
          },
          ariaLabel: "Telegram",
          link: "https://t.me/+G_YxIv-XTFlhNWU0",
        },
        { icon: "discord", link: "https://discord.gg/KZWaFtPZJf" },
      ],

      editLink: {
        pattern:
          "https://github.com/software-mansion/scarb/tree/main/website/:path",
        text: "Edit this page on GitHub",
      },

      search: {
        provider: "local",
      },
    },

    sitemap: {
      hostname: absoluteBase,
    },

    markdown: {
      languages: [syntaxes.cairo],
    },
  }),
);

function p(text, link) {
  return { text, link };
}
