import syntax from "cairo-tm-grammar";

export const cairo = {
  ...syntax,

  // NODE: The Cairo syntax uses capital-case for language name,
  //   which is interpreted differently by Shiki, hence this override.
  name: "cairo",
};
