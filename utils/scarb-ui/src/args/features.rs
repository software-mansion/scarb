use clap::Parser;

/// [`clap`] structured arguments that provide features selection.
#[derive(Parser, Clone, Debug)]
pub struct FeaturesSpec {
    /// Comma separated list of features to activate.
    #[arg(short = 'F', long, value_delimiter = ',', env = "SCARB_FEATURES")]
    pub features: Vec<String>,

    /// Activate all available features.
    #[arg(
        long,
        default_value_t = false,
        env = "SCARB_ALL_FEATURES",
        conflicts_with = "no_default_features"
    )]
    pub all_features: bool,

    /// Do not activate the `default` feature.
    #[arg(
        long,
        default_value_t = false,
        env = "SCARB_NO_DEFAULT_FEATURES",
        conflicts_with = "all_features"
    )]
    pub no_default_features: bool,
}

impl super::ToEnvVars for FeaturesSpec {
    fn to_env_vars(self) -> Vec<(String, String)> {
        let mut env = vec![("SCARB_FEATURES".to_string(), self.features.join(","))];
        if self.all_features {
            env.push((
                "SCARB_ALL_FEATURES".to_string(),
                self.all_features.to_string(),
            ));
        }
        if self.no_default_features {
            env.push((
                "SCARB_NO_DEFAULT_FEATURES".to_string(),
                self.no_default_features.to_string(),
            ));
        }
        env
    }
}
