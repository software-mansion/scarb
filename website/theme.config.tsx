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
    if (asPath !== "/") {
      return {
        titleTemplate: "%s – Scarb",
      };
    }
  },
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
