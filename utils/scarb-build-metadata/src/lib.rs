#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct CommitHash {
    pub full: &'static str,
    pub short: &'static str,
}

pub const SCARB_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const SCARB_COMMIT_HASH: Option<CommitHash> = match (
    option_env!("SCARB_COMMIT_HASH"),
    option_env!("SCARB_COMMIT_SHORT_HASH"),
) {
    (Some(full), Some(short)) => Some(CommitHash { full, short }),
    (None, None) => None,
    _ => panic!("Either SCARB_COMMIT_HASH or SCARB_COMMIT_SHORT_HASH is missing."),
};
pub const SCARB_COMMIT_DATE: Option<&str> = option_env!("SCARB_COMMIT_DATE");

pub const CAIRO_VERSION: &str = env!("SCARB_CAIRO_VERSION");
pub const CAIRO_COMMIT_HASH: Option<CommitHash> = match (
    option_env!("SCARB_CAIRO_COMMIT_HASH"),
    option_env!("SCARB_CAIRO_SHORT_COMMIT_HASH"),
) {
    (Some(full), Some(short)) => Some(CommitHash { full, short }),
    (None, None) => None,
    _ => panic!("Either SCARB_CAIRO_COMMIT_HASH or SCARB_CAIRO_SHORT_COMMIT_HASH is missing."),
};
pub const CAIRO_COMMIT_REV: &str = env!("SCARB_CAIRO_COMMIT_REV");

#[cfg(test)]
mod tests {
    use semver::{BuildMetadata, Prerelease, Version};

    /// Checks that package version in [`Scarb.toml`] is exactly the same as the version of Cairo,
    /// because this project is tightly coupled with it.
    #[test]
    fn scarb_version_is_bound_to_cairo_version() {
        let nightly_build = crate::SCARB_VERSION.contains("nightly");

        let normalize = |v| {
            let mut v = Version::parse(v).unwrap();
            v.build = BuildMetadata::EMPTY;
            if nightly_build {
                v.pre = Prerelease::EMPTY;
            }
            v.to_string()
        };

        let scarb_version = normalize(crate::SCARB_VERSION);
        let cairo_version = normalize(crate::CAIRO_VERSION);
        assert_eq!(scarb_version, cairo_version);
    }
}
