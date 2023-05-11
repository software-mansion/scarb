import { Footer } from "@/components/footer";
import { HeaderLogo } from "@/components/logo";
import { NextSeoProps } from "next-seo";
import { useRouter } from "next/router";
import { DocsThemeConfig } from "nextra-theme-docs";

const config: DocsThemeConfig = {
  logo: <HeaderLogo />,
  primaryHue: { light: 239, dark: 204 },
  // banner: {
  //   key: "scarb-0.2.0-alpha.2-released",
  //   text: (
  //     <a href="https://github.com/software-mansion/scarb/releases/tag/v0.2.0-alpha.2">
  //       🎉 The second alpha of Scarb v0.2 is released. Read more →
  //     </a>
  //   ),
  // },
  project: {
    link: "https://github.com/software-mansion/scarb",
  },
  docsRepositoryBase:
    "https://github.com/software-mansion/scarb/tree/main/website",
  useNextSeoProps,
  head: Head,
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
    text: <Footer />,
  },
};

function useLinkFn(): (path: string) => string {
  const { basePath } = useRouter();
  return (path: string) => `${basePath}${path}`;
}

function useNextSeoProps(): NextSeoProps {
  const { asPath } = useRouter();
  return {
    titleTemplate: asPath === "/" ? "%s" : "%s – Scarb",
    description:
      "Scarb is a build toolchain and package manager for Cairo and Starknet ecosystems.",
    openGraph: {
      type: "website",
      images: [
        {
          url: "https://docs.swmansion.com/scarb/og-image.png",
          type: "image/png",
          width: 1280,
          height: 640,
          alt: "Scarb is a build toolchain and package manager for Cairo and Starknet ecosystems.",
        },
      ],
    },
    twitter: {
      cardType: "summary_large_image",
      site: "@swmansionxyz",
      handle: "@jajakobyly",
    },
  };
}

function Head() {
  const link = useLinkFn();
  return (
    <>
      <meta httpEquiv="Content-Language" content="en" />
      <link rel="manifest" href={link("/manifest.json")} />
      <link rel="icon" href={link("/favicon.ico")} sizes="any" />
      <link rel="icon" href={link("/favicon.svg")} type="image/svg+xml" />
      <link rel="apple-touch-icon" href={link("/apple-touch-icon.png")} />
      <meta name="apple-mobile-web-app-title" content="Scarb" />
    </>
  );
}

export default config;
