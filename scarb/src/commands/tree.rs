
use std::fmt::Write;

use anyhow::Result;
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::CrateId;
use clap::Parser;
use colored::Colorize;
use indoc::indoc;

use crate::compiler::get_main_crate_ids_from_project;
use crate::core::{Workspace};
use crate::get_cairo_db;
use crate::help;



#[derive(Debug, Parser)]
#[command(about, long_about = indoc! {"
    Display a tree visualization of a dependency graph.

    This command will display a tree of dependencies to the terminal.
"})]
pub struct TreeCommand {

    #[arg(long)]
    depth: Option<usize>,


    #[arg(short, long)]
    duplicates: bool,

    
    #[arg(long)]
    no_dedupe: bool,


    #[arg(long, default_value = "indent")]
    prefix: PrefixKind,

    /// Format string for each package
    #[arg(short, long, default_value = "{p}")]
    format: String,
}

#[derive(Debug, Parser, Clone, Copy)]
enum PrefixKind {
  
    Indent,
  
    Depth,
 
    None,
}

impl std::str::FromStr for PrefixKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "indent" => Ok(PrefixKind::Indent),
            "depth" => Ok(PrefixKind::Depth),
            "none" => Ok(PrefixKind::None),
            _ => Err(anyhow::anyhow!("Invalid prefix kind: {}", s)),
        }
    }
}

impl TreeCommand {
    pub fn run(self, workspace: &Workspace) -> Result<()> {
        let db = get_cairo_db(workspace)?;
        let main_crates = get_main_crate_ids_from_project(&db, workspace.manifest().origin_package())?;

        println!("{} v{} ({})", 
            workspace.manifest().origin_package().name.bold(), 
            workspace.manifest().origin_package().version,
            workspace.manifest().origin_package().id.path().to_string_lossy().italic()
        );

        let mut seen_crates = std::collections::HashSet::new();
        
        // For each main crate, print its dependency tree
        for &main_crate_id in &main_crates {
            self.print_dependencies(
                &db, 
                main_crate_id, 
                0, 
                &mut seen_crates, 
                self.depth,
                !self.no_dedupe,
                self.prefix,
            )?;
        }

        Ok(())
    }

    fn print_dependencies(
        &self,
        db: &dyn FilesGroup,
        crate_id: CrateId,
        depth: usize,
        seen_crates: &mut std::collections::HashSet<CrateId>,
        max_depth: Option<usize>,
        dedupe: bool,
        prefix_kind: PrefixKind,
    ) -> Result<()> {
        // Check if we've reached the maximum depth
        if let Some(max) = max_depth {
            if depth > max {
                return Ok(());
            }
        }

        // Skip if we've already seen this crate and dedupe is enabled
        let is_duplicate = !seen_crates.insert(crate_id);
        if is_duplicate && dedupe {
            return Ok(());
        }

        // Get the dependencies of this crate
        let crate_info = db.crate_info(crate_id);
        let dependencies = crate_info.dependencies.clone();

        // Construct the prefix based on the chosen prefix kind
        let prefix = match prefix_kind {
            PrefixKind::Indent => {
                let mut s = String::new();
                if depth > 0 {
                    write!(s, "{}", "│   ".repeat(depth - 1))?;
                    write!(s, "└── ")?;
                }
                s
            }
            PrefixKind::Depth => format!("{}{} ", " ".repeat(4), depth),
            PrefixKind::None => String::new(),
        };

        // Print this crate's information
        let crate_name = crate_info.name.as_str();
        let formatted = self.format.replace("{p}", crate_name);
        
        // Mark duplicates
        let display_str = if is_duplicate && dedupe {
            format!("{}{} (*)", prefix, formatted)
        } else {
            format!("{}{}", prefix, formatted)
        };
        
        println!("{}", display_str);

        // Continue with dependencies
        for dep in dependencies {
            self.print_dependencies(
                db, 
                dep.id, 
                depth + 1, 
                seen_crates, 
                max_depth,
                dedupe,
                prefix_kind,
            )?;
        }

        Ok(())
    }
}