import Image from "next/image";
import { ReactElement } from "react";
import swm from "../public/swm.svg";

export function SWM(): ReactElement {
  return (
    <a
      href="https://swmansion.com/"
      target="_blank"
      rel="noopener noreferrer"
      className="opacity-25 transition-opacity hover:opacity-100 focus:opacity-100 active:opacity-100"
    >
      <Image src={swm} alt="Software Mansion" />
    </a>
  );
}
