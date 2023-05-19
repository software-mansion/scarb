import { Link } from "nextra-theme-docs";
import { ComponentProps, ReactElement, ReactNode } from "react";

interface BigLinkProps extends ComponentProps<typeof Link> {
  text?: ReactNode;
}

export function BigLink({ text, ...linkProps }: BigLinkProps): ReactElement {
  return (
    <p className="mt-6 text-center text-lg leading-7 first:mt-0">
      <Link {...linkProps}>{text} →</Link>
    </p>
  );
}
