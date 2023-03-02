import React, { ReactElement } from "react";

export function Logo(): ReactElement {
  return (
    <span className="bg-gradient-to-br from-sea-80 to-blue-80 bg-clip-text text-xl font-extrabold uppercase tracking-wide text-transparent dark:from-darksea-60 dark:to-darkblue-60">
      Scarb
    </span>
  );
}
