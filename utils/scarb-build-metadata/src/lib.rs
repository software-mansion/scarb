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
    use semver::Version;

    /// Checks that package version in [`Scarb.toml`] is (almost) the same as the version of Cairo,
    /// because this project is tightly coupled with it.
    ///
    /// Things we mandate:
    /// 1. Major and minor parts **must** be the same.
    /// 2. Patch part **must** be greater than (in rare occasions) or equal to (usually)
    ///    the patch part of Cairo.
    /// 3. Pre-release part **must** be greater than or equal to the pre-release part of Cairo,
    ///    **unless** it is a nightly Scarb build or patch part is strictly greater than the patch
    ///    part of Cairo.
    /// 4. Build parts are ignored.
    #[test]
    #[ignore]
    fn scarb_version_is_bound_to_cairo_version() {
        let scarb = Version::parse(crate::SCARB_VERSION).unwrap();
        let cairo = Version::parse(crate::CAIRO_VERSION).unwrap();

        let msg = format!("versions not in sync:\nscarb {scarb}\ncairo {cairo}");

        assert_eq!(
            (scarb.major, scarb.minor),
            (cairo.major, cairo.minor),
            "{msg}"
        );
        assert!(scarb.patch >= cairo.patch, "{msg}");
        assert!(
            scarb.pre.contains("nightly") || scarb.patch > cairo.patch || scarb.pre >= cairo.pre,
            "{msg}"
        );
    }
}
