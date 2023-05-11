import { default as NextLink } from "next/link";
import { ComponentPropsWithoutRef, ReactElement } from "react";

export type LinkProps = ComponentPropsWithoutRef<typeof NextLink>;

export function Link({ children, ...props }: LinkProps): ReactElement {
  return (
    <NextLink
      {...props}
      className="nx-text-primary-600 decoration-from-font [text-underline-position:from-font] hover:underline focus:underline"
    >
      {children}
    </NextLink>
  );
}
