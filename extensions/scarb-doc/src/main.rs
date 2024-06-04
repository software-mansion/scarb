use anyhow::Result;
use clap::Parser;

use scarb_metadata::MetadataCommand;
use scarb_ui::args::PackagesFilter;

use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::CrateLongId;

use compilation::get_project_config;
use types::Crate;

mod compilation;
mod types;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(flatten)]
    packages_filter: PackagesFilter,
}

macro_rules! print_names {
    ($label:expr, $var:expr) => {
        println!(
            "{}: {:?}",
            $label,
            $var.iter().map(|x| &x.item_data.name).collect::<Vec<_>>()
        );
    };
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

    let main_crate_id = db.intern_crate(CrateLongId::Real(package_metadata.name.clone().into()));
    let crate_ = Crate::new(db, main_crate_id);

    print_module(&crate_.root_module);

    Ok(())
}

fn print_module(module: &types::Module) {
    println!("Module: {}", module.full_path);
    println!(
        "Submodules      : {:?}",
        module
            .submodules
            .iter()
            .map(|x| &x.full_path)
            .collect::<Vec<_>>()
    );
    print_names!("Constants       ", module.constants);
    print_names!("Uses            ", module.uses);
    print_names!("Free Functions  ", module.free_functions);
    print_names!("Structs         ", module.structs);
    print_names!("Enums           ", module.enums);
    print_names!("Type Aliases    ", module.type_aliases);
    print_names!("Impl Aliases    ", module.impl_aliases);
    print_names!("Traits          ", module.traits);
    print_names!("Impls           ", module.impls);
    print_names!("Extern Types    ", module.extern_types);
    print_names!("Extern Functions", module.extern_functions);

    for submodule in &module.submodules {
        println!();
        print_module(submodule);
    }
}
