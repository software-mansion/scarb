use crate::core::Workspace;
use crate::SCARB_ENV;
use std::collections::HashMap;
use std::ffi::OsString;

pub const EXTERNAL_CMD_PREFIX: &str = "scarb-";
pub const SCARB_MANIFEST_PATH_ENV: &str = "SCARB_MANIFEST_PATH";

/// Defines env vars passed to external subcommands.
pub fn get_env_vars(ws: &Workspace<'_>) -> anyhow::Result<HashMap<OsString, OsString>> {
    let config = ws.config();
    Ok(HashMap::from_iter([
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
            "SCARB_TARGET_DIR".into(),
            ws.target_dir().path_unchecked().into(),
        ),
        ("SCARB_PROFILE".into(), config.profile().as_str().into()),
        (
            "SCARB_UI_VERBOSITY".into(),
            config.ui().verbosity().to_string().into(),
        ),
        (SCARB_ENV.into(), config.app_exe()?.into()),
    ]))
}
