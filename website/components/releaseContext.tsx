import { Release } from "@/data/github";
import { useSSG } from "nextra/ssg";
import { createContext, PropsWithChildren, useContext } from "react";

const ReleaseContext = createContext<Release | undefined>(undefined);

export function useRelease(): Release {
  const release = useContext(ReleaseContext);
  if (!release) {
    throw new Error("missing WithRelease in component tree");
  }
  return release;
}

export function WithRelease({
  kind,
  children,
}: PropsWithChildren<{ kind: "stable" | "preview" }>) {
  const release = useSSG(kind);
  if (release) {
    return (
      <ReleaseContext.Provider value={release}>
        {children}
      </ReleaseContext.Provider>
    );
  } else {
    return (
      <p className="nx-mt-6 nx-leading-7 first:nx-mt-0">
        There is no current {kind} release at this moment.
      </p>
    );
  }
}

export function ReleaseVersion() {
  const release = useRelease();
  return <>{release.version}</>;
}
