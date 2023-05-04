import mermaid from "mermaid";
import { ReactElement, useEffect, useId, useState } from "react";

let initialized = false;

export function Mermaid({ chart }: { chart: string }): ReactElement {
  // Thanks to: https://github.com/mariansimecek/remark-mermaid-nextra/blob/main/src/Mermaid.tsx
  const id = useId();
  const [svg, setSvg] = useState("");

  useEffect(() => {
    // Perform exactly once
    if (!initialized) {
      initialized = true;
      mermaid.initialize({
        fontFamily: "inherit",
      });
    }

    // see https://mermaid-js.github.io/mermaid/#/theming?id=theme-variables-reference-table
    mermaid
      .render(
        id.replace(/[^a-zA-Z]+/g, ""), // strip special chars from useId
        `${chart}` // apply theme and supply chart
      )
      .then(({ svg }) => {
        setSvg(svg);
      })
      .catch((error) => {
        // eslint-disable-next-line no-console -- show error
        console.error("Error while rendering mermaid", error);
      });
  }, [id, chart]);

  return (
    <div
      className="mt-6 flex justify-center"
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  );
}

export default Mermaid;
