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
