pub enum SupportedPlatform {
    Aarch64AppleDarwin,
    Aarch64UnknownLinuxGnu,
    X8664AppleDarwin,
    X8664PcWindowsMsvc,
    X8664UnknownLinuxGnu,
}

impl SupportedPlatform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Aarch64AppleDarwin => "aarch64-apple-darwin",
            Self::Aarch64UnknownLinuxGnu => "aarch64-unknown-linux-gnu",
            Self::X8664AppleDarwin => "x86_64-apple-darwin",
            Self::X8664PcWindowsMsvc => "x86_64-pc-windows-msvc",
            Self::X8664UnknownLinuxGnu => "x86_64-unknown-linux-gnu",
        }
    }

    pub fn variants() -> &'static [SupportedPlatform; 5] {
        &[
            Self::Aarch64AppleDarwin,
            Self::Aarch64UnknownLinuxGnu,
            Self::X8664AppleDarwin,
            Self::X8664PcWindowsMsvc,
            Self::X8664UnknownLinuxGnu,
        ]
    }
}
