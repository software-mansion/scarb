import { Terminal } from "@/components/terminal";
import Link from "next/link";
import { ReactElement } from "react";

export function Jumbo(): ReactElement {
  return (
    <div className="flex flex-col items-center justify-center gap-20 py-16 text-center lg:flex-row">
      <div className="flex max-w-[30em] flex-col space-y-8">
        <p className="text-4xl font-light text-blue-60 dark:text-darkblue-20">
          The build toolchain and package manager for Cairo and Starknet
          ecosystems
        </p>
        <Download />
      </div>
      <Terminal
        lines={[
          "scarb init --name hello_world",
          <>
            scarb add quaireaux_math --git{" "}
            <span style={{ userSelect: "none" }}>
              \<br />
              &nbsp;&nbsp;&nbsp;&nbsp;
            </span>
            https://github.com/keep-starknet-strange/quaireaux.git
          </>,
          "scarb build",
        ]}
      />
    </div>
  );
}

function Download(): ReactElement {
  return (
    <Link
      href="/download"
      className="mx-auto flex bg-blue-80 px-6 py-4 text-xl font-medium text-white transition hover:bg-blue-100 focus:bg-blue-100"
    >
      <DownloadIcon />
      Download
    </Link>
  );
}

function DownloadIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className="mr-[.5em] w-[1em]"
    >
      <path
        d="M20 15V18C20 19.1046 19.1046 20 18 20H6C4.89543 20 4 19.1046 4 18L4 15M8 11L12 15M12 15L16 11M12 15V3"
        className="stroke-white"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
