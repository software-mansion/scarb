/*
Language: Cairo
Website: https://www.cairo-lang.org
Category: common, system
*/

/** @type LanguageFn */

export default function(hljs) {
  const UNDERSCORE_IDENT_RE = hljs.UNDERSCORE_IDENT_RE;
  const KEYWORDS = [
    "as",
    "break",
    "const",
    "continue",
    "else",
    "enum",
    "extern",
    "false",
    "fn",
    "if",
    "impl",
    "implicits",
    "let",
    "loop",
    "match",
    "mod",
    "mut",
    "nopanic",
    "of",
    "pub",
    "ref",
    "return",
    "self",
    "struct",
    "super",
    "trait",
    "true",
    "type",
    "use",
    "while",
  ];
  const LITERALS = [
    "true",
    "false"
  ];
  const BUILTINS = [
    // functions
    "assert ",
    "panic ",
    // traits
    "Copy",
    "Send",
    "Serde",
    "Sized",
    "Sync",
    "Drop",
    "Fn",
    "FnMut",
    "FnOnce",
    "ToOwned",
    "Clone",
    "Debug",
    "PartialEq",
    "PartialOrd",
    "Eq",
    "Ord",
    "AsRef",
    "AsMut",
    "Into",
    "From",
    "Default",
    "Iterator",
    "Extend",
    "IntoIterator",
    "DoubleEndedIterator",
    "ExactSizeIterator",
    "SliceConcatExt",
    "ToString",
    // macros
    "assert!",
    "assert_eq!",
    "assert_ne!",
    "assert_lt!",
    "assert_le!",
    "assert_gt!",
    "assert_ge!",
    "format!",
    "write!",
    "writeln!",
    "get_dep_component!",
    "get_dep_component_mut!",
    "component!",
    "consteval_int!",
    "array!",
    "panic!",
    "print!",
    "println!",
  ];
  const TYPES = [
    "felt252",
    "i8",
    "i16",
    "i32",
    "i64",
    "i128",
    "u8",
    "u16",
    "u32",
    "u64",
    "u128",
    "usize",
    "bool",
    "Box",
    "Option",
    "Result",
  ];
  return {
    name: "Cairo",
    aliases: [ "cairo" ],
    keywords: {
      $pattern: hljs.IDENT_RE + "!?",
      type: TYPES,
      keyword: KEYWORDS,
      literal: LITERALS,
      built_in: BUILTINS,
    },
    illegal: "</",
    contains: [
      hljs.C_LINE_COMMENT_MODE,
      hljs.COMMENT("/\\*", "\\*/", { contains: [ "self" ] }),
      hljs.inherit(hljs.QUOTE_STRING_MODE, {
        begin: /b?"/,
        illegal: null,
      }),
      {
        className: "string",
        variants: [
          { begin: /b?r(#*)"(.|\n)*?"\1(?!#)/ },
          { begin: /b?'\\?(x\w{2}|u\w{4}|U\w{8}|.)'/ },
        ],
      },
      {
        className: "symbol",
        begin: /'[a-zA-Z_][a-zA-Z0-9_]*/,
      },
      {
        className: "number",
        variants: [
          { begin: "\\b0b([01_]+)" },
          { begin: "\\b0o([0-7_]+)" },
          { begin: "\\b0x([A-Fa-f0-9_]+)" },
          { begin:
              "\\b(\\d[\\d_]*(\\.[0-9_]+)?([eE][+-]?[0-9_]+)?)", },
        ],
        relevance: 0,
      },
      {
        begin: [
          /fn/,
          /\s+/,
          UNDERSCORE_IDENT_RE
        ],
        className: {
          1: "keyword",
          3: "title.function",
        },
      },
      {
        className: "meta",
        begin: "#!?\\[",
        end: "\\]",
        contains: [
          {
            className: "string",
            begin: /"/,
            end: /"/,
            contains: [ hljs.BACKSLASH_ESCAPE ],
          },
        ],
      },
      {
        begin: [
          /let/,
          /\s+/,
          /(?:mut\s+)?/,
          UNDERSCORE_IDENT_RE
        ],
        className: {
          1: "keyword",
          3: "keyword",
          4: "variable",
        },
      },
      // must come before impl/for rule later
      {
        begin: [
          /for/,
          /\s+/,
          UNDERSCORE_IDENT_RE,
          /\s+/,
          /in/
        ],
        className: {
          1: "keyword",
          3: "variable",
          5: "keyword",
        },
      },
      {
        begin: [
          /type/,
          /\s+/,
          UNDERSCORE_IDENT_RE
        ],
        className: {
          1: "keyword",
          3: "title.class",
        },
      },
      {
        begin: [
          /(?:trait|enum|struct|union|impl|for)/,
          /\s+/,
          UNDERSCORE_IDENT_RE,
        ],
        className: {
          1: "keyword",
          3: "title.class",
        },
      },
      {
        begin: hljs.IDENT_RE + "::",
        keywords: {
          keyword: "Self",
          built_in: BUILTINS,
          type: TYPES,
        },
      },
      {
        className: "punctuation",
        begin: "->",
      },
    ],
  };
}
