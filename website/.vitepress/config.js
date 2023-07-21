import { defineConfig } from "vitepress";
import { withMermaid } from "vitepress-plugin-mermaid"; // https://vitepress.dev/reference/site-config

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
      ],
    },
    {
      text: "Reference",
      items: [
        p("Compilation model", "/docs/reference/compilation-model"),
        p("Conditional compilation", "/docs/reference/conditional-compilation"),
        p("Global directories", "/docs/reference/global-directories"),
        p("Manifest", "/docs/reference/manifest"),
        p("Workspaces", "/docs/reference/workspaces"),
        p("Profiles", "/docs/reference/profiles"),
        p("Scripts", "/docs/reference/scripts"),
        p("Specifying dependencies", "/docs/reference/specifying-dependencies"),
        p("Targets", "/docs/reference/targets"),
      ],
    },
    {
      text: "Writing extensions",
      items: [
        p("JSON output", "/docs/writing-extensions/json-output"),
        p("Scarb crate", "/docs/writing-extensions/scarb-crate"),
        p("Scarb metadata", "/docs/writing-extensions/scarb-metadata"),
        p("Subcommands", "/docs/writing-extensions/subcommands"),
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
      ],
    },
  ],
};
export default withMermaid(
  defineConfig({
    title: "Scarb",
    description:
      "Scarb is a build toolchain and package manager for Cairo and Starknet ecosystems.",
    lang: "en-US",
    base: "/scarb/",

    head: [
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
        {
          property: "og:image",
          content: "https://docs.swmansion.com/scarb/og-image.png",
        },
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
    ],

    lastUpdated: true,

    themeConfig: {
      logo: {
        light: "logo-light.svg",
        dark: "logo-dark.svg",
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
      hostname: "https://docs.swmansion.com/scarb/",
    },
  }),
);

function p(text, link) {
  return { text, link };
}
