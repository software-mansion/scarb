use anyhow::Result;
use clap::Parser;

macro_rules! command {
    ($enum_name:ident ( $($module:ident,)+ )) => {
        $(mod $module;)+

        #[derive(::clap::Subcommand)]
        #[allow(non_camel_case_types)]
        enum $enum_name {
            $($module(crate::$module::Args),)+
        }

        impl $enum_name {
            fn main(self) -> ::anyhow::Result<()> {
                match self {
                    $(Self::$module(args) => crate::$module::main(args),)+
                }
            }
        }
    }
}

command!(Command(
    create_archive,
    get_nightly_version,
    list_binaries,
    nightly_release_notes,
    set_cairo_version,
    set_scarb_version,
    verify_archive,
));

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

fn main() -> Result<()> {
    Args::parse().command.main()
}
