import cairoSyntax from "cairo-tm-grammar";
import witSyntax from "./wit.tmLanguage.json" with { type: "json" };

export const cairo = {
  ...cairoSyntax,

  // NOTE: The Cairo syntax uses capital-case for language name,
  //   which is interpreted differently by Shiki, hence this override.
  name: "cairo",
};

export const wit = {
  ...witSyntax,

  // Same reason as with Cairo.
  name: "wit",
};
