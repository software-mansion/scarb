use anyhow::{bail, Result};

/// Check the base requirements for a package name.
pub fn validate_package_name(name: &str, what: &str) -> Result<()> {
    if name.is_empty() {
        bail!("empty string cannot be used as {what}");
    }

    if name == "_" {
        bail!("underscore cannot be used as {what}");
    }

    let mut chars = name.chars();

    // Validate first letter.
    if let Some(ch) = chars.next() {
        // A specific error for a potentially common case.
        if ch.is_ascii_digit() {
            bail!(
                "the name `{name}` cannot be used as a {what}, \
                names cannot start with a digit"
            );
        }

        if !(ch.is_ascii_alphabetic() || ch == '_') {
            bail!(
                "invalid character `{ch}` in {what}: `{name}`, \
                the first character must be an ASCII letter or underscore"
            )
        }
    }

    // Validate rest.
    for ch in chars {
        if !(ch.is_ascii_alphanumeric() || ch == '_') {
            bail!(
                "invalid character `{ch}` in {what}: `{name}`, \
                characters must be ASCII letter, ASCII numbers or underscore"
            )
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::validate_package_name;

    #[test_case("foo")]
    #[test_case("_bar")]
    fn validate_correct_package_name(name: &str) {
        assert!(validate_package_name(name, "P").is_ok())
    }

    #[test_case("" => "empty string cannot be used as P"; "empty string")]
    #[test_case("_" => "underscore cannot be used as P"; "underscore")]
    #[test_case("1" => "the name `1` cannot be used as a P, names cannot start with a digit")]
    #[test_case("123" => "the name `123` cannot be used as a P, names cannot start with a digit")]
    #[test_case("0foo" => "the name `0foo` cannot be used as a P, names cannot start with a digit")]
    #[test_case("fo-o" => "invalid character `-` in P: `fo-o`, characters must be ASCII letter, ASCII numbers or underscore")]
    fn validate_incorrect_package_name(name: &str) -> String {
        format!("{}", validate_package_name(name, "P").unwrap_err())
    }
}
