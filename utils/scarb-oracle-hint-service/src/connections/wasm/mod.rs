use crate::connection::Connection;
use crate::connections::wasm::codec::{decode_from_cairo, encode_to_cairo};
use crate::protocol::Protocol;
use anyhow::{Context, Result, anyhow};
use starknet_core::types::Felt;
use std::sync::LazyLock;
use wasmtime::component::{Component, Instance, Linker, ResourceTable, Val};
use wasmtime::{Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

mod codec;

/// The `wasm` protocol loads a WebAssembly component from a file and allows
/// invoking its exported functions by name. The selector maps to the exported
/// function name.
pub struct Wasm {
    store: Store<HostState>,
    instance: Instance,
}

impl Protocol for Wasm {
    const SCHEME: &'static str = "wasm";

    #[tracing::instrument]
    fn connect(path: &str) -> Result<Box<dyn Connection + 'static>> {
        static ENGINE: LazyLock<Engine> = LazyLock::new(Engine::default);

        let mut linker = Linker::new(&ENGINE);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker).expect("failed to link wasip2");

        let component = Component::from_file(&ENGINE, path)
            .with_context(|| format!("failed to load wasm component from: {path}"))?;

        let mut store = Store::new(&ENGINE, HostState::default());
        let instance = linker
            .instantiate(&mut store, &component)
            .context("failed to instantiate wasm component (missing imports?)")?;

        Ok(Box::new(Wasm { store, instance }))
    }
}

impl Connection for Wasm {
    fn call(&mut self, selector: &str, calldata: &[Felt]) -> Result<Vec<Felt>> {
        let func = self
            .instance
            .get_func(&mut self.store, selector)
            .ok_or_else(|| anyhow!("unsupported selector: {selector}"))?;

        let func_params: Vec<codec::Ty> = func
            .params(&self.store)
            .into_iter()
            .map(|(_, ty)| ty.try_into())
            .collect::<Result<_>>()?;

        let params = decode_from_cairo(&func_params, calldata)?;
        let mut results = vec![Val::U8(0); func.results(&self.store).len()];
        func.call(&mut self.store, &params, &mut results)?;
        let results = encode_to_cairo(&results);
        func.post_return(&mut self.store)?;
        results
    }
}

struct HostState {
    ctx: WasiCtx,
    table: ResourceTable,
}

impl Default for HostState {
    fn default() -> Self {
        Self {
            // TODO(#2629): Preopen some directory if the runtime wants to.
            // TODO(#2627): Redirect stderr to tracing logs.
            // TODO(#2625): Implement permissions system to allow users to limit these caps.
            ctx: WasiCtx::builder()
                .inherit_stdio()
                .allow_blocking_current_thread(true)
                .inherit_env()
                .inherit_network()
                .allow_ip_name_lookup(true)
                .build(),
            table: ResourceTable::new(),
        }
    }
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}
