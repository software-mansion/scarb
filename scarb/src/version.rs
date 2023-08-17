//! Version information about Scarb and Cairo.

use std::fmt;
use std::fmt::Write;

use indoc::formatdoc;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use scarb_build_metadata::{
    CommitHash, CAIRO_COMMIT_HASH, CAIRO_VERSION, SCARB_COMMIT_DATE, SCARB_COMMIT_HASH,
    SCARB_VERSION,
};

/// Scarb's version.
#[derive(Serialize, Deserialize, Debug)]
pub struct VersionInfo {
    pub version: &'static str,
    pub commit_info: Option<CommitInfo>,
    pub cairo: CairoVersionInfo,
    pub sierra: SierraVersionInfo,
}

/// Cairo's version.
#[derive(Serialize, Deserialize, Debug)]
pub struct CairoVersionInfo {
    pub version: &'static str,
    pub commit_info: Option<CommitInfo>,
}

/// Sierra version.
#[derive(Serialize, Deserialize, Debug)]
pub struct SierraVersionInfo {
    pub version: &'static str,
}

/// Information about the Git repository where the crate was built from.
#[derive(Serialize, Deserialize, Debug)]
pub struct CommitInfo {
    pub short_commit_hash: &'static str,
    pub commit_hash: &'static str,
    pub commit_date: Option<&'static str>,
}

impl CommitInfo {
    fn from_commit_hash(
        commit_hash: Option<CommitHash>,
        commit_date: Option<&'static str>,
    ) -> Option<Self> {
        commit_hash.map(|h| Self {
            short_commit_hash: h.short,
            commit_hash: h.full,
            commit_date,
        })
    }
}

impl VersionInfo {
    pub fn short(&self) -> String {
        display_version_and_commit_info(self.version, &self.commit_info, None)
    }

    pub fn long(&self) -> String {
        formatdoc!(
            r#"
                {short}
                cairo: {cairo}
                sierra: {sierra}
            "#,
            short = self.short(),
            cairo = self.cairo.short(),
            sierra = self.sierra.short(),
        )
    }
}

impl CairoVersionInfo {
    pub fn short(&self) -> String {
        display_version_and_commit_info(
            self.version,
            &self.commit_info,
            Some("cairo-lang-compiler"),
        )
    }
}

impl SierraVersionInfo {
    pub fn short(&self) -> &str {
        self.version
    }
}

fn display_version_and_commit_info(
    version: &str,
    commit_info: &Option<CommitInfo>,
    crate_name: Option<&str>,
) -> String {
    let mut text = version.to_string();
    if let Some(commit_info) = commit_info {
        write!(&mut text, " ({commit_info})").unwrap();
    } else if let Some(crate_name) = crate_name {
        write!(
            &mut text,
            " (https://crates.io/crates/{crate_name}/{version})"
        )
        .unwrap();
    }
    text
}

impl fmt::Display for CommitInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.short_commit_hash)?;

        if let Some(date) = &self.commit_date {
            write!(f, " {}", date)?;
        }

        Ok(())
    }
}

/// Get information about Scarb's version.
pub fn get() -> VersionInfo {
    let commit_info = CommitInfo::from_commit_hash(SCARB_COMMIT_HASH, SCARB_COMMIT_DATE);

    let cairo = {
        CairoVersionInfo {
            version: CAIRO_VERSION,
            commit_info: CommitInfo::from_commit_hash(CAIRO_COMMIT_HASH, None),
        }
    };

    static SIERRA_VERSION: Lazy<String> = Lazy::new(|| {
        cairo_lang_starknet::compiler_version::current_sierra_version_id().to_string()
    });

    let sierra = SierraVersionInfo {
        version: &SIERRA_VERSION,
    };

    VersionInfo {
        version: SCARB_VERSION,
        commit_info,
        cairo,
        sierra,
    }
}
