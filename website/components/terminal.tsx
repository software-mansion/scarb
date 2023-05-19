import { ReactElement, ReactNode } from "react";

export interface TerminalProps {
  lines: ReactNode[];
}

export function Terminal({ lines }: TerminalProps): ReactElement {
  return (
    <div className="h-full overflow-hidden rounded-lg bg-black-40 px-5 pb-6 pt-4 text-left font-mono text-sm leading-normal text-white-40 subpixel-antialiased shadow-xl">
      <div className="top mb-2 flex gap-2">
        <div className="h-3 w-3 rounded-full bg-red-100" />
        <div className="h-3 w-3 rounded-full bg-yellow-100" />
        <div className="h-3 w-3 rounded-full bg-green-100" />
      </div>
      <div className="mt-4 flex flex-col">
        {lines.map((prompt, i) => (
          <p key={i} className="typing flex-1 items-center">
            <span className="text-green-100" style={{ userSelect: "none" }}>
              ${" "}
            </span>
            {prompt}
            <br />
          </p>
        ))}
      </div>
    </div>
  );
}
