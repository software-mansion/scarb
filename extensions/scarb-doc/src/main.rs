use anyhow::Result;
use camino::Utf8PathBuf;
use clap::Parser;
use std::collections::HashMap;

use cairo_lang_compiler::{db::RootDatabase, project::setup_single_file_project};

use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::ids::{LookupItemId, ModuleItemId, NamedLanguageElementId};

use cairo_lang_semantic::db::SemanticGroup;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    crate_path: Utf8PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut db = RootDatabase::default();

    let crate_id = setup_single_file_project(&mut db, args.crate_path.as_std_path())?;

    let crate_modules = db.crate_modules(crate_id);

    let mut item_documentation = HashMap::new();

    for module_id in crate_modules.iter() {
        let module_items = db.module_items(*module_id).unwrap();

        for item in module_items.iter() {
            let item_doc = db.get_item_documentation(LookupItemId::ModuleItem(*item));
            item_documentation.insert(LookupItemId::ModuleItem(*item), item_doc);

            if let ModuleItemId::Trait(trait_id) = *item {
                let trait_items_names = db.trait_required_item_names(trait_id).unwrap();

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
            LookupItemId::ModuleItem(item_id) => item_id.name(&db),
            LookupItemId::TraitItem(item_id) => item_id.name(&db),
            LookupItemId::ImplItem(item_id) => item_id.name(&db),
        };
        println!("{:?}: {:?} -> {:?}", name, item_id, doc);
    }

    Ok(())
}
