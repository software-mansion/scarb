import { Logo } from "@/components/logo";
import { SWM } from "@/components/swm";
import { useRouter } from "next/router";
import { DocsThemeConfig } from "nextra-theme-docs";

const config: DocsThemeConfig = {
  logo: <Logo />,
  primaryHue: { light: 239, dark: 204 },
  // banner: {
  //   key: "scarb-0.1-released",
  //   text: (
  //     <a href="https://github.com/software-mansion/scarb/releases/tag/v0.1.0">
  //       🎉 Scarb v0.1 is released. Read more →
  //     </a>
  //   ),
  // },
  project: {
    link: "https://github.com/software-mansion/scarb",
  },
  docsRepositoryBase:
    "https://github.com/software-mansion/scarb/tree/main/website",
  useNextSeoProps() {
    const { asPath } = useRouter();
    return {
      titleTemplate: asPath === "/" ? "%s" : "%s – Scarb",
      description:
        "Scarb is a build toolchain and package manager for Cairo and Starknet ecosystems.",
      twitter: {
        cardType: "summary_large_image",
        site: "@swmansionxyz",
        handle: "@jajakobyly",
      },
    };
  },
  head: (
    <>
      <meta httpEquiv="Content-Language" content="en" />
      <meta name="msapplication-TileColor" content="#fff" />
      <meta name="apple-mobile-web-app-title" content="Scarb" />
    </>
  ),
  editLink: {
    text: "Edit this page on GitHub →",
  },
  feedback: {
    content: null,
    // content: 'Question? Give us feedback →',
  },
  sidebar: {
    toggleButton: true,
  },
  footer: {
    text: (
      <div style={{ margin: "0 auto" }}>
        <SWM />
      </div>
    ),
  },
};

export default config;
