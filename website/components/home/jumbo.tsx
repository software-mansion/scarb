import { Terminal } from "@/components/terminal";
import { ReactElement } from "react";

export function Jumbo(): ReactElement {
  return (
    <div className="flex flex-col items-center justify-center gap-20 py-16 text-center lg:flex-row">
      <div className="flex max-w-[30em] flex-col space-y-8">
        <p className="text-4xl font-light text-blue-60 dark:text-darkblue-20">
          The build toolchain and package manager for Cairo and Starknet
          ecosystems
        </p>
      </div>
      <Terminal
        lines={[
          "scarb init --name hello_world",
          <>
            scarb add alexandria --git{" "}
            <span style={{ userSelect: "none" }}>
              \<br />
              &nbsp;&nbsp;&nbsp;&nbsp;
            </span>
            https://github.com/keep-starknet-strange/alexandria.git
          </>,
          "scarb build",
        ]}
      />
    </div>
  );
}
