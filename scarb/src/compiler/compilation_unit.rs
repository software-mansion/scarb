use std::fmt::Write;

use crate::core::manifest::Target;
use crate::core::Package;

/// An object that has enough information so that Scarb knows how to build it.
#[derive(Clone, Debug)]
pub struct CompilationUnit {
    /// The Scarb [`Package`] to be build.
    pub package: Package,
    /// Information about the specific target to build, out of the possible targets in `package`.
    pub target: Target,
    /// Collection of all [`Package`]s needed to provide as _crate roots_ to
    /// the Cairo compiler in order to build `package`.
    pub components: Vec<Package>,
}

impl CompilationUnit {
    pub fn is_sole_for_package(&self) -> bool {
        self.package.manifest.targets.len() >= 2
    }

    pub fn has_custom_name(&self) -> bool {
        self.target.kind.name() != self.package.id.name.as_str()
    }

    pub fn name(&self) -> String {
        let mut string = String::new();

        if self.is_sole_for_package() {
            write!(&mut string, "{}", self.target.kind.name()).unwrap();

            if self.has_custom_name() {
                write!(&mut string, "({})", self.target.name).unwrap();
            }

            write!(&mut string, " ").unwrap();
        }

        write!(&mut string, "{}", self.package.id).unwrap();

        string
    }
}
