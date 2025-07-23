use clap::Parser;

/// Profile specifier.
#[derive(Parser, Clone, Debug)]
#[group(multiple = true)]
pub struct ProfileSpec {
    /// Specify the profile to use by name.
    #[arg(short = 'P', long, env = "SCARB_PROFILE")]
    pub profile: Option<String>,
    /// Use release profile.
    #[arg(long, hide_short_help = true, group = "ProfileShortcuts")]
    pub release: bool,
    /// Use dev profile.
    #[arg(long, hide_short_help = true, group = "ProfileShortcuts")]
    pub dev: bool,
}

impl ProfileSpec {
    const RELEASE: &'static str = "release";
    const DEV: &'static str = "dev";

    /// Return the specified profile as a string.
    pub fn specified(&self) -> Option<String> {
        match &self {
            Self { release: true, .. } => Some(Self::RELEASE.to_string()),
            Self { dev: true, .. } => Some(Self::DEV.to_string()),
            Self {
                profile: Some(profile),
                ..
            } => Some(profile.to_string()),
            _ => None,
        }
    }
}

impl super::ToEnvVars for ProfileSpec {
    fn to_env_vars(self) -> Vec<(String, String)> {
        self.specified()
            .map(|profile| (String::from("SCARB_PROFILE"), profile))
            .into_iter()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::ProfileSpec;
    use crate::args::ToEnvVars;

    #[test]
    fn can_read_dev() {
        assert_eq!(
            ProfileSpec {
                dev: true,
                profile: None,
                release: false,
            }
            .specified(),
            Some("dev".to_string())
        );
    }

    #[test]
    fn can_read_release() {
        assert_eq!(
            ProfileSpec {
                dev: false,
                profile: None,
                release: true,
            }
            .specified(),
            Some("release".to_string())
        );
    }

    #[test]
    fn can_read_profile() {
        assert_eq!(
            ProfileSpec {
                dev: false,
                profile: Some("custom".to_string()),
                release: false,
            }
            .specified(),
            Some("custom".to_string())
        );
    }

    #[test]
    fn shortcut_takes_precedence() {
        assert_eq!(
            ProfileSpec {
                dev: true,
                profile: Some("custom".to_string()),
                release: false,
            }
            .specified(),
            Some("dev".to_string())
        );
    }

    #[test]
    fn convert_to_env_vars() {
        let profile_spec = ProfileSpec {
            dev: false,
            profile: Some("custom".to_string()),
            release: false,
        };
        assert_eq!(
            profile_spec.to_env_vars(),
            vec![("SCARB_PROFILE".to_string(), "custom".to_string())]
        );
    }
}
