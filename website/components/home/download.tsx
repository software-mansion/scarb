import { ReleaseVersion, WithRelease } from "@/components/releaseContext";
import { platform } from "@/data/platform";
import dynamic from "next/dynamic";
import { Link } from "nextra-theme-docs";
import { Pre } from "nextra/components";

export function Download() {
  return (
    <div className="rounded-2xl border border-white-80 bg-white-20 p-12 text-center text-blue-100 shadow-md shadow-white-60 dark:border-darkblue-40 dark:bg-darkblue-120 dark:text-darksea-20 dark:shadow-darkblue-120 md:col-span-2">
      <WithRelease kind="stable">
        <DownloadPlatformSpecific />
      </WithRelease>
    </div>
  );
}

const DownloadPlatformSpecific = dynamic(
  () =>
    Promise.resolve(() => {
      switch (platform()) {
        case "macos":
        case "linux":
          return <DownloadUnix />;
        default:
          return (
            <p className="text-lg">
              <RestOfLinks all />
            </p>
          );
      }
    }),
  { ssr: false }
);

function DownloadUnix() {
  return (
    <>
      <p className="text-xl">
        Run the following in your terminal, then follow the onscreen
        instructions.
      </p>
      <Pre hasCopyCode className="whitespace-normal text-center">
        <code>
          {
            "curl --proto '=https' --tlsv1.2 -sSf https://docs.swmansion.com/scarb/install.sh | sh"
          }
        </code>
      </Pre>
      <span className="text-sm">
        You appear to be running macOS or Linux. This command will install the
        latest stable version of Scarb:&nbsp;
        <ReleaseVersion />. <RestOfLinks />
      </span>
    </>
  );
}

function RestOfLinks({ all = false }: { all?: boolean }) {
  return (
    <>
      For {all ? "all" : "other"} Scarb versions, platforms or installation
      methods or general help, go to the{" "}
      <Link href="./download">download page</Link>.
    </>
  );
}
