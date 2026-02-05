use crate::SCARB_ENV;
use crate::core::Config;
use camino::Utf8PathBuf;
use std::collections::HashMap;
use std::ffi::OsString;

pub const EXTERNAL_CMD_PREFIX: &str = "scarb-";
pub const SCARB_MANIFEST_PATH_ENV: &str = "SCARB_MANIFEST_PATH";

/// Defines env vars passed to external subcommands.
pub fn get_env_vars(
    config: &Config,
    target_dir: Option<Utf8PathBuf>,
) -> anyhow::Result<HashMap<OsString, OsString>> {
    let mut vars: Vec<(OsString, OsString)> = vec![
        ("PATH".into(), config.dirs().path_env()),
        (
            "SCARB_CACHE".into(),
            config.dirs().cache_dir.path_unchecked().into(),
        ),
        (
            "SCARB_CONFIG".into(),
            config.dirs().config_dir.path_unchecked().into(),
        ),
        ("SCARB_LOG".into(), config.log_filter_directive().into()),
        (
            SCARB_MANIFEST_PATH_ENV.into(),
            config.manifest_path().into(),
        ),
        (
            "SCARB_OUTPUT_JSON".into(),
            as_truthy_env_value(config.ui().output_format().is_json()),
        ),
        ("SCARB_PROFILE".into(), config.profile().as_str().into()),
        (
            "SCARB_UI_VERBOSITY".into(),
            config.ui().verbosity().to_string().into(),
        ),
        (
            "SCARB_NO_PROC_MACROS".into(),
            (!config.proc_macro_repository().load_proc_macros())
                .to_string()
                .into(),
        ),
        (
            "SCARB_NO_PREBUILT_PROC_MACROS".into(),
            (!config.load_prebuilt_proc_macros()).to_string().into(),
        ),
        (SCARB_ENV.into(), config.app_exe()?.into()),
    ];
    if let Some(target_dir) = target_dir {
        vars.push(("SCARB_TARGET_DIR".into(), target_dir.into()));
    }
    Ok(HashMap::from_iter(vars))
}

pub fn as_truthy_env_value(value: bool) -> OsString {
    OsString::from(if value { "true" } else { "false" })
}
