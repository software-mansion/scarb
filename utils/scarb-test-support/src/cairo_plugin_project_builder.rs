use crate::fsx;
use crate::project_builder::ProjectBuilder;
use assert_fs::fixture::{FileWriteStr, PathChild};
use camino::Utf8PathBuf;
use indoc::{formatdoc, indoc};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;

static CAIRO_LANG_MACRO_PATH_V2: LazyLock<String> = LazyLock::new(|| {
    let path = fsx::canonicalize(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../plugins/")
            .join("cairo-lang-macro"),
    )
    .unwrap();
    serde_json::to_string(&path).unwrap()
});

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CairoPluginProjectVersion {
    V1,
    #[default]
    V2,
}

pub struct CairoPluginProjectBuilder {
    project: ProjectBuilder,
    name: String,
    src: HashMap<Utf8PathBuf, String>,
    deps: Vec<String>,
    macro_version: CairoPluginProjectVersion,
}

impl CairoPluginProjectBuilder {
    pub fn start() -> Self {
        Self {
            project: ProjectBuilder::start(),
            name: Default::default(),
            src: Default::default(),
            deps: Default::default(),
            macro_version: CairoPluginProjectVersion::default(),
        }
    }

    pub fn start_v1() -> Self {
        Self {
            project: ProjectBuilder::start(),
            name: Default::default(),
            src: Default::default(),
            deps: Default::default(),
            macro_version: CairoPluginProjectVersion::V1,
        }
    }

    pub fn scarb_project(mut self, mutate: impl FnOnce(ProjectBuilder) -> ProjectBuilder) -> Self {
        self.project = mutate(self.project);
        self
    }

    pub fn name(mut self, name: impl ToString) -> Self {
        self.name = name.to_string();
        self.project = self.project.name(name.to_string());
        self
    }

    pub fn version(mut self, version: impl ToString) -> Self {
        self.project = self.project.version(&version.to_string());
        self
    }

    pub fn src(mut self, path: impl Into<Utf8PathBuf>, source: impl ToString) -> Self {
        self.src.insert(path.into(), source.to_string());
        self
    }

    pub fn lib_rs(self, source: impl ToString) -> Self {
        self.src("src/lib.rs", source.to_string())
    }

    pub fn add_dep(mut self, dep: impl ToString) -> Self {
        self.deps.push(dep.to_string());
        self
    }

    fn render_manifest(&self) -> String {
        let macro_lib_path = CAIRO_LANG_MACRO_PATH_V2.to_string();
        let deps = self.deps.join("\n");
        let name = self.name.clone();
        let macro_lib_version_req = match self.macro_version {
            CairoPluginProjectVersion::V1 => "\"0.1\"".to_string(),
            CairoPluginProjectVersion::V2 => {
                format!("{{ path = {macro_lib_path}, version = \"0.2.0-rc.0\" }}")
            }
        };
        formatdoc! {r#"
                [package]
                name = "{name}"
                version = "0.1.0"
                edition = "2021"
                publish = false

                [lib]
                crate-type = ["cdylib"]

                [dependencies]
                cairo-lang-macro = {macro_lib_version_req}
                {deps}
                "#}
    }

    pub fn just_code(&self, t: &impl PathChild) {
        t.child("Cargo.toml")
            .write_str(self.render_manifest().as_str())
            .unwrap();
        for (path, source) in &self.src {
            t.child(path).write_str(source).unwrap();
        }
    }

    pub fn build(&self, t: &impl PathChild) {
        self.project.just_manifest(t);
        self.just_code(t);
    }

    pub fn add_primitive_token_dep(self) -> Self {
        self.add_dep(r#"cairo-lang-primitive-token = "1""#)
    }

    pub fn add_cairo_lang_parser_dep(self) -> Self {
        self.add_dep(r#"cairo-lang-parser = "2.11""#)
    }

    pub fn add_cairo_lang_syntax_dep(self) -> Self {
        self.add_dep(r#"cairo-lang-syntax = "2.11""#)
    }

    pub fn default_v1() -> Self {
        let default_name = "some";
        let default_code = indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro};
        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            ProcMacroResult::new(token_stream)
        }
        "#};
        Self::start_v1()
            .name(default_name)
            .scarb_project(|b| {
                b.name(default_name)
                    .version("1.0.0")
                    .manifest_extra("[cairo-plugin]")
            })
            .lib_rs(default_code)
    }
}

impl Default for CairoPluginProjectBuilder {
    fn default() -> Self {
        let default_name = "some";
        let default_code = indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, CAIRO_LANG_MACRO_API_VERSION, attribute_macro};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            assert!(CAIRO_LANG_MACRO_API_VERSION == unsafe { std::num::NonZeroU8::new_unchecked(2)} );
            ProcMacroResult::new(token_stream)
        }
        "#};
        Self::start()
            .name(default_name)
            .scarb_project(|b| {
                b.name(default_name)
                    .version("1.0.0")
                    .manifest_extra(indoc! {r#"
                        [cairo-plugin]
                    "#})
            })
            .lib_rs(default_code)
    }
}
