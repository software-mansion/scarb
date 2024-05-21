use anyhow::Result;
use clap::Parser;

use scarb_metadata::MetadataCommand;
use scarb_ui::args::PackagesFilter;

use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::CrateLongId;
use cairo_lang_starknet::starknet_plugin_suite;

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
        b.with_plugin_suite(starknet_plugin_suite());
        b.build()?
    };

    let main_crate_id = db.intern_crate(CrateLongId::Real(package_metadata.name.clone().into()));
    let crate_ = Crate::new(db, main_crate_id);

    print_module(&crate_.root_module);
    println!("{crate_:?}");

    Ok(())
}

fn print_module(module: &types::Module) {
    println!("Module: {:?}", module.full_path);
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
    for st in module.structs.iter() {
        println!("    Struct {:?}", st.item_data.name);
        print_names!("        Members", st.members);
    }

    print_names!("Enums           ", module.enums);
    for enu in module.enums.iter() {
        println!("    Enum {:?}", enu.item_data.name);
        print_names!("        Variants", enu.variants);
    }
    print_names!("Type Aliases    ", module.type_aliases);
    print_names!("Impl Aliases    ", module.impl_aliases);

    print_names!("Traits          ", module.traits);
    for tr in module.traits.iter() {
        println!("    Trait {:?}", tr.item_data.name);
        print_names!("        Trait constants", tr.trait_constants);
        print_names!("        Trait types    ", tr.trait_types);
        print_names!("        Trait functions", tr.trait_functions);
    }

    print_names!("Impls           ", module.impls);
    for imp in module.impls.iter() {
        println!("    Impl {:?}", imp.item_data.name);
        print_names!("        Impl types     ", imp.impl_types);
        print_names!("        Impl constants ", imp.impl_constants);
        print_names!("        Impl functions ", imp.impl_functions);
    }
    print_names!("Extern Types    ", module.extern_types);
    print_names!("Extern Functions", module.extern_functions);

    for submodule in &module.submodules {
        println!();
        print_module(submodule);
    }
}
