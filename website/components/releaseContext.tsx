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
    return null;
  }
}

export function ReleaseVersion() {
  const release = useRelease();
  return <>{release.tag}</>;
}
