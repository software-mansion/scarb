use crate::fsx;
use crate::project_builder::ProjectBuilder;
use assert_fs::fixture::{FileWriteStr, PathChild};
use camino::Utf8PathBuf;
use indoc::{formatdoc, indoc};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;

static CAIRO_LANG_MACRO_PATH: LazyLock<String> = LazyLock::new(|| {
    let path = fsx::canonicalize(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../plugins/")
            .join("cairo-lang-macro"),
    )
    .unwrap();
    serde_json::to_string(&path).unwrap()
});

pub struct CairoPluginProjectBuilder {
    project: ProjectBuilder,
    name: String,
    src: HashMap<Utf8PathBuf, String>,
    deps: Vec<String>,
}

impl CairoPluginProjectBuilder {
    pub fn start() -> Self {
        Self {
            project: ProjectBuilder::start(),
            name: Default::default(),
            src: Default::default(),
            deps: Default::default(),
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
        let macro_lib_path = CAIRO_LANG_MACRO_PATH.to_string();
        let deps = self.deps.join("\n");
        let name = self.name.clone();
        formatdoc! {r#"
                [package]
                name = "{name}"
                version = "0.1.0"
                edition = "2021"
                publish = false

                [lib]
                crate-type = ["cdylib"]

                [dependencies]
                cairo-lang-macro = {{ path = {macro_lib_path}, version = "0.1.0" }}
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
}

impl Default for CairoPluginProjectBuilder {
    fn default() -> Self {
        let default_name = "some";
        let default_code = indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            ProcMacroResult::new(token_stream)
        }
        "#};
        Self::start()
            .name(default_name)
            .scarb_project(|b| {
                b.name(default_name)
                    .version("1.0.0")
                    .manifest_extra(r#"[cairo-plugin]"#)
            })
            .lib_rs(default_code)
    }
}
