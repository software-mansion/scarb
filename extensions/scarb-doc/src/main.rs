use anyhow::Result;
use cairo_lang_compiler::project::{AllCratesConfig, ProjectConfig, ProjectConfigContent};
use cairo_lang_filesystem::cfg::{Cfg, CfgSet};
use cairo_lang_filesystem::db::{CrateSettings, Edition, ExperimentalFeaturesConfig, FilesGroup};
use clap::Parser;
use scarb_metadata::{
    CompilationUnitComponentMetadata, CompilationUnitMetadata, Metadata, MetadataCommand,
    PackageId, PackageMetadata,
};
use scarb_ui::args::PackagesFilter;
use smol_str::{SmolStr, ToSmolStr};
use std::collections::HashMap;
use std::path::PathBuf;

use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_utils::ordered_hash_map::OrderedHashMap;

use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::ids::{LookupItemId, ModuleItemId, NamedLanguageElementId};

use cairo_lang_filesystem::ids::Directory;
use cairo_lang_semantic::db::SemanticGroup;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(flatten)]
    packages_filter: PackagesFilter,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let metadata = MetadataCommand::new().inherit_stderr().exec()?;
    let package_metadata = args.packages_filter.match_one(&metadata)?;

    let project_config = get_project_config(&metadata, &package_metadata);

    let db = &mut {
        let mut b = RootDatabase::builder();
        b.with_project_config(project_config);
        b.build()?
    };

    let crate_ids = db.crates();
    let main_crate_id = crate_ids.into_iter().next().unwrap();

    let crate_modules = db.crate_modules(main_crate_id);

    let mut item_documentation = HashMap::new();

    for module_id in crate_modules.iter() {
        let module_items = db.module_items(*module_id).unwrap();

        for item in module_items.iter() {
            let item_doc = db.get_item_documentation(LookupItemId::ModuleItem(*item));
            item_documentation.insert(LookupItemId::ModuleItem(*item), item_doc);

            if let ModuleItemId::Trait(trait_id) = *item {
                let trait_items_names = db.trait_item_names(trait_id).unwrap();

                for trait_item_name in trait_items_names.into_iter() {
                    let trait_item_id = db
                        .trait_item_by_name(trait_id, trait_item_name)
                        .unwrap()
                        .unwrap();

                    let doc = db.get_item_documentation(LookupItemId::TraitItem(trait_item_id));
                    item_documentation.insert(LookupItemId::TraitItem(trait_item_id), doc);
                }
            }
        }
    }

    for (item_id, doc) in item_documentation.iter() {
        let name = match item_id {
            LookupItemId::ModuleItem(item_id) => item_id.name(db),
            LookupItemId::TraitItem(item_id) => item_id.name(db),
            LookupItemId::ImplItem(item_id) => item_id.name(db),
        };
        println!("{:?}: {:?} -> {:?}", name, item_id, doc);
    }

    Ok(())
}

fn get_project_config(metadata: &Metadata, package_metadata: &PackageMetadata) -> ProjectConfig {
    let compilation_unit_metadata =
        package_lib_compilation_unit(metadata, package_metadata.id.clone())
            .expect("Failed to find compilation unit for package");
    let corelib = get_corelib(compilation_unit_metadata);
    let dependencies = get_dependencies(compilation_unit_metadata);
    let crates_config = get_crates_config(metadata, compilation_unit_metadata);

    ProjectConfig {
        base_path: package_metadata.root.clone().into(),
        corelib: Some(Directory::Real(corelib.source_root().into())),
        content: ProjectConfigContent {
            crate_roots: dependencies,
            crates_config,
        },
    }
}

fn package_lib_compilation_unit(
    metadata: &Metadata,
    package_id: PackageId,
) -> Option<&CompilationUnitMetadata> {
    metadata
        .compilation_units
        .iter()
        .find(|m| m.package == package_id && m.target.kind == LIB_TARGET_KIND)
}

fn get_corelib(
    compilation_unit_metadata: &CompilationUnitMetadata,
) -> &CompilationUnitComponentMetadata {
    compilation_unit_metadata
        .components
        .iter()
        .find(|du| du.name == CORELIB_CRATE_NAME)
        .expect("Corelib could not be found")
}

fn get_dependencies(
    compilation_unit_metadata: &CompilationUnitMetadata,
) -> OrderedHashMap<SmolStr, PathBuf> {
    compilation_unit_metadata
        .components
        .iter()
        .filter(|du| du.name != "core")
        .map(|cu| {
            (
                cu.name.to_smolstr(),
                cu.source_root().to_owned().into_std_path_buf(),
            )
        })
        .collect()
}

fn get_crates_config(
    metadata: &Metadata,
    compilation_unit_metadata: &CompilationUnitMetadata,
) -> AllCratesConfig {
    let crates_config: OrderedHashMap<SmolStr, CrateSettings> = compilation_unit_metadata
        .components
        .iter()
        .map(|component| {
            let pkg = metadata.get_package(&component.package).unwrap_or_else(|| {
                panic!(
                    "Failed to find = {} package",
                    &component.package.to_string()
                )
            });
            (
                SmolStr::from(&component.name),
                get_crate_settings_for_package(
                    pkg,
                    component.cfg.as_ref().map(|cfg_vec| build_cfg_set(cfg_vec)),
                ),
            )
        })
        .collect();

    AllCratesConfig {
        override_map: crates_config,
        ..Default::default()
    }
}

fn get_crate_settings_for_package(
    package: &PackageMetadata,
    cfg_set: Option<CfgSet>,
) -> CrateSettings {
    let edition = package
        .edition
        .clone()
        .map_or(Edition::default(), |edition| {
            let edition_value = serde_json::Value::String(edition);
            serde_json::from_value(edition_value).unwrap()
        });

    let experimental_features = ExperimentalFeaturesConfig {
        negative_impls: package
            .experimental_features
            .contains(&String::from("negative_impls")),
        coupons: package
            .experimental_features
            .contains(&String::from("coupons")),
    };

    CrateSettings {
        edition,
        cfg_set,
        experimental_features,
    }
}

fn build_cfg_set(cfg: &[scarb_metadata::Cfg]) -> CfgSet {
    CfgSet::from_iter(cfg.iter().map(|cfg| {
        serde_json::to_value(cfg)
            .and_then(serde_json::from_value::<Cfg>)
            .expect("Cairo's `Cfg` must serialize identically as Scarb Metadata's `Cfg`.")
    }))
}

const LIB_TARGET_KIND: &str = "lib";
const CORELIB_CRATE_NAME: &str = "core";
