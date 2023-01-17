use snapbox::cmd::{cargo_bin, Command};

pub fn scarb_command() -> Command {
    let cache = assert_fs::TempDir::new().unwrap();
    let config = assert_fs::TempDir::new().unwrap();

    Command::new(cargo_bin!("scarb"))
        .env("SCARB_LOG", "scarb=trace")
        .env("SCARB_CACHE", cache.path())
        .env("SCARB_CONFIG", config.path())
}
