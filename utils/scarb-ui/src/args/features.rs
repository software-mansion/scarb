use clap::Parser;
use smol_str::SmolStr;

/// Features
#[derive(Parser, Clone, Debug)]
pub struct FeaturesSpec {
    /// Comma separated list of features to activate.
    #[arg(short = 'F', long, value_delimiter = ',', env = "SCARB_FEATURES")]
    pub features: Vec<SmolStr>,

    /// Activate all available features.
    #[arg(long, default_value_t = false, env = "SCARB_ALL_FEATURES")]
    pub all_features: bool,

    /// Do not activate the `default` feature.
    #[arg(long, default_value_t = false, env = "SCARB_NO_DEFAULT_FEATURES")]
    pub no_default_features: bool,
}
