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

/// Commit hash corresponding to cairo compiler semver version.
pub const CAIRO_COMMIT_REV: &str = env!("SCARB_CAIRO_COMMIT_REV");
/// Optional path to corelib from local cargo cache.
///
/// If `None`, that the corelib should be attempted to be downloaded from Cairo
/// repository on GitHub.
pub const SCARB_CORELIB_LOCAL_PATH: Option<&str> = option_env!("SCARB_CORELIB_LOCAL_PATH");

#[cfg(test)]
mod tests {
    use semver::{BuildMetadata, Prerelease, Version};

    /// Checks that package version in [`Scarb.toml`] is exactly the same as the version of Cairo,
    /// because this project is tightly coupled with it.
    #[test]
    #[ignore = "2.8.3 release with cairo 2.8.2"]
    fn scarb_version_is_bound_to_cairo_version() {
        let mut scarb = Version::parse(crate::SCARB_VERSION).unwrap();
        let mut cairo = Version::parse(crate::CAIRO_VERSION).unwrap();

        scarb.build = BuildMetadata::EMPTY;
        cairo.build = BuildMetadata::EMPTY;

        if scarb.pre.contains("nightly") {
            scarb.pre = Prerelease::EMPTY;
            cairo.pre = Prerelease::EMPTY;
        }

        assert_eq!(
            (scarb.major, scarb.minor, scarb.patch),
            (cairo.major, cairo.minor, cairo.patch),
            "versions not in sync:\nscarb {scarb}\ncairo {cairo}"
        );
        assert!(
            scarb.pre >= cairo.pre,
            "versions not in sync:\nscarb {scarb}\ncairo {cairo}"
        );
    }
}
