


pub mod tree;


#[derive(Debug, Parser)]
pub enum Commands {
    // ... other commands
    #[command(name = "tree")]
    Tree(tree::TreeCommand),
    // ... other commands
}

// Then in the match statement where commands are executed:
pub fn execute_command(cmd: Commands, ctx: CommandContext) -> Result<()> {
    match cmd {
        // ... other commands
        Commands::Tree(cmd) => cmd.run(&ctx.workspace?),
        // ... other commands
    }
}