import Link from "next/link";
import { ComponentProps, ReactElement, ReactNode } from "react";

interface BigLinkProps extends ComponentProps<typeof Link> {
  text?: ReactNode;
}

export function BigLink({ text, ...linkProps }: BigLinkProps): ReactElement {
  return (
    <p className="nx-mt-6 nx-leading-7 first:nx-mt-0 text-center text-lg">
      <Link
        rel="noreferrer"
        target="_blank"
        className="nx-text-primary-600 decoration-from-font [text-underline-position:from-font] hover:underline focus:underline"
        {...linkProps}
      >
        {text} →
      </Link>
    </p>
  );
}
