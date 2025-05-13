pub struct ScarbArgs {
    /// Path to Scarb.toml.
    #[arg(long, env = "SCARB_MANIFEST_PATH", hide_short_help = true)]
    pub manifest_path: Option<Utf8PathBuf>,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: VerbositySpec,

    /// Print machine-readable output in NDJSON format.
    #[arg(long, env = "SCARB_OUTPUT_JSON")]
    pub json: bool,

    /// Run without accessing the network.
    #[arg(long, env = "SCARB_OFFLINE", hide_short_help = true)]
    pub offline: bool,

    /// Directory for all cache data stored by Scarb.
    #[arg(
        long,
        env = "SCARB_CACHE",
        value_name = "DIRECTORY",
        hide_short_help = true
    )]
    pub global_cache_dir: Option<Utf8PathBuf>,

    /// Directory for global Scarb configuration files.
    #[arg(
        long,
        env = "SCARB_CONFIG",
        value_name = "DIRECTORY",
        hide_short_help = true
    )]
    pub global_config_dir: Option<Utf8PathBuf>,

    /// Directory for all generated artifacts.
    #[arg(
        long,
        env = "SCARB_TARGET_DIR",
        value_name = "DIRECTORY",
        hide_short_help = true
    )]
    pub target_dir: Option<Utf8PathBuf>,

    /// Specify the profile to use.
    #[command(flatten)]
    pub profile_spec: ProfileSpec,

    /// Subcommand and its arguments.
    #[command(subcommand)]
    pub command: Command,
}

#[doc(hidden)]
impl ScarbArgs {
    /// Construct [`OutputFormat`] value from these arguments.
    pub fn output_format(&self) -> OutputFormat {
        if self.json {
            OutputFormat::Json
        } else {
            OutputFormat::default()
        }
    }

    pub fn get_builtin_subcommands() -> BTreeMap<String, Option<String>> {
        Self::command()
            .get_subcommands()
            .map(|sub| {
                let name = sub.get_name().to_string();
                let about = sub.get_about().map(|s| s.to_string());
                (name, about)
            })
            .collect()
    }
} 