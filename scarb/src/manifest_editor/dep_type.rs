pub trait SectionArgs {
    fn dev(&self) -> bool;
}

#[derive(Clone, Debug)]
pub enum DepType {
    Normal(String),
    Dev(String),
}

impl DepType {
    pub fn as_str(&self) -> &str {
        match self {
            DepType::Normal(s) | DepType::Dev(s) => s,
        }
    }
}
impl Default for DepType {
    fn default() -> Self {
        Self::Normal(String::from("dependencies"))
    }
}

impl DepType {
    pub fn from_section(section_args: &impl SectionArgs) -> DepType {
        if section_args.dev() {
            DepType::Dev(String::from("dev-dependencies"))
        } else {
            DepType::Normal(String::from("dependencies"))
        }
    }
}
