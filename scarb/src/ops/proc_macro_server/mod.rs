use crate::compiler::plugin::proc_macro::ProcMacroHost;
use anyhow::Result;
use connection::Connection;

mod connection;

pub fn start_proc_macro_server(proc_macros: ProcMacroHost) -> Result<()> {
    let connection = Connection::new();

    //TODO

    connection.join();

    Ok(())
}
