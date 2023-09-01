//! Helpers for validating and checking names.

use cairo_lang_filesystem::db::CORELIB_CRATE_NAME;

/// Checks if name is a Cairo keyword
pub fn is_keyword(name: &str) -> bool {
    [
        "as",
        "assert",
        "break",
        "const",
        "continue",
        "do",
        "dyn",
        "else",
        "enum",
        "extern",
        "false",
        "fn",
        "for",
        "hint",
        "if",
        "impl",
        "implicits",
        "in",
        "let",
        "loop",
        "macro",
        "match",
        "mod",
        "move",
        "mut",
        "nopanic",
        "of",
        "pub",
        "ref",
        "return",
        "self",
        "static",
        "static_assert",
        "struct",
        "super",
        "trait",
        "true",
        "try",
        "type",
        "typeof",
        "unsafe",
        "use",
        "where",
        "while",
        "with",
        "yield",
    ]
    .contains(&name)
}

/// Checks if name is restricted on Windows platforms
pub fn is_windows_restricted(name: &str) -> bool {
    [
        "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
        "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ]
    .contains(&name)
}

/// Checks if name equals `core` or `starknet`
pub fn is_internal(name: &str) -> bool {
    [CORELIB_CRATE_NAME, "starknet"].contains(&name)
}
