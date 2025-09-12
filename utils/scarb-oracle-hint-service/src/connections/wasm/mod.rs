use crate::connection::Connection;
use crate::connections::wasm::codec::{decode_from_cairo, encode_to_cairo};
use crate::protocol::Protocol;
use anyhow::{Context, Result, bail};
use starknet_core::types::Felt;
use std::collections::HashMap;
use std::io::{self, Write};
use std::pin::Pin;
use std::sync::LazyLock;
use std::sync::{Arc, Mutex};
use std::task::{Context as TaskContext, Poll};
use tokio::io::AsyncWrite;
use tracing::debug;
use tracing::debug_span;
use wasmtime::component::types::ComponentItem;
use wasmtime::component::{
    Component, ComponentExportIndex, Func, Instance, Linker, ResourceTable, Val,
};
use wasmtime::{Engine, Store};
use wasmtime_wasi::cli::{IsTerminal, StdoutStream};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

mod codec;

/// A custom stderr writer that forwards output to tracing in real-time
#[derive(Clone)]
struct TracingStderrWriter {
    buffer: Arc<Mutex<Vec<u8>>>,
    span: tracing::Span,
}

impl TracingStderrWriter {
    fn new() -> Self {
        let span = debug_span!("err");
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            span,
        }
    }
}

impl Write for TracingStderrWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let _span = self.span.enter();

        let mut buffer = self.buffer.lock().unwrap();
        buffer.extend_from_slice(buf);

        // Process complete lines immediately
        let mut start = 0;
        while let Some(end) = buffer[start..].iter().position(|&b| b == b'\n') {
            let line_end = start + end;
            if let Ok(line) = std::str::from_utf8(&buffer[start..line_end]) {
                debug!("{}", line);
            }
            start = line_end + 1;
        }

        // Keep remaining incomplete line in buffer
        if start > 0 {
            buffer.drain(0..start);
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let _span = self.span.enter();

        let mut buffer = self.buffer.lock().unwrap();
        if !buffer.is_empty() {
            if let Ok(text) = std::str::from_utf8(&buffer) {
                debug!("{}", text);
            }
            buffer.clear();
        }
        Ok(())
    }
}

impl std::fmt::Debug for TracingStderrWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TracingStderrWriter").finish()
    }
}

impl IsTerminal for TracingStderrWriter {
    fn is_terminal(&self) -> bool {
        false
    }
}

impl StdoutStream for TracingStderrWriter {
    fn async_stream(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        Box::new(self.clone())
    }
}

impl AsyncWrite for TracingStderrWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut TaskContext<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(self.write(buf))
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(self.flush())
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}

/// Maps fully qualified export names to their indices; i.e.:
/// `naked:adder/add@0.1.0#add` -> `0`.
type FullyQualifiedFuncs = HashMap<String, ComponentExportIndex>;

/// Maps unqualified export names to their indices (if unambiguous) or a list of ambiguous fully
/// qualified paths; i.e.: `add` -> `Ok(0)` or `sub` -> `Err(["a:a#sub", "b:b#sub"])`.
type ShortFuncs = HashMap<String, Result<ComponentExportIndex, Vec<String>>>;

/// The `wasm` protocol loads a WebAssembly component from a file and allows
/// invoking its exported functions by name. The selector maps to the exported
/// function name.
pub struct Wasm {
    store: Store<HostState>,
    instance: Instance,
    fully_qualified_funcs: FullyQualifiedFuncs,
    short_funcs: ShortFuncs,
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

        let (fully_qualified_funcs, short_funcs) = dbg!(Self::collect_funcs(&component, &ENGINE));

        Ok(Box::new(Wasm {
            store,
            instance,
            fully_qualified_funcs,
            short_funcs,
        }))
    }
}

impl Connection for Wasm {
    fn call(&mut self, selector: &str, calldata: &[Felt]) -> Result<Vec<Felt>> {
        let func = self.search_component_func(selector)?;

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
        self.store.data_mut().flush_stderr();
        results
    }
}

impl Wasm {
    fn search_component_func(&mut self, selector: &str) -> Result<Func> {
        let index = if let Some(index) = self.fully_qualified_funcs.get(selector) {
            *index
        } else if let Some(index) = self.short_funcs.get(selector) {
            match index {
                Ok(index) => *index,
                Err(ambiguities) => {
                    let ambiguities = {
                        let mut v = ambiguities.to_vec();
                        v.sort();
                        v.join(", ")
                    };
                    bail!(
                        "multiple exports named: {selector}\n\
                         note: possible matches are: {ambiguities}"
                    );
                }
            }
        } else {
            let available = {
                let mut v = self
                    .fully_qualified_funcs
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>();
                v.sort();
                v.join(", ")
            };
            bail!(
                "no exported func in component named: {selector}\n\
                 note: available funcs are: {available}"
            );
        };

        Ok(self
            .instance
            .get_func(&mut self.store, index)
            .expect("unable to get export index that we computed"))
    }

    fn collect_funcs(component: &Component, engine: &Engine) -> (FullyQualifiedFuncs, ShortFuncs) {
        let mut fully_qualified_funcs = Default::default();
        let mut short_funcs = Default::default();

        fn visit(
            component: &Component,
            engine: &Engine,
            item: ComponentItem,
            basename: Vec<String>,
            fully_qualified_funcs: &mut FullyQualifiedFuncs,
            short_funcs: &mut ShortFuncs,
        ) {
            let push_name = |name: &str| {
                let mut basename = basename.clone();
                basename.push(name.to_owned());
                basename
            };

            match item {
                ComponentItem::ComponentFunc(_) => {
                    let name = basename
                        .last()
                        .expect("expected non-empty basename")
                        .clone();
                    let fqn = basename.join("/");

                    let index = basename
                        .iter()
                        .fold(None, |instance, name| {
                            component.get_export_index(instance.as_ref(), name)
                        })
                        .expect("export has at least one name");

                    short_funcs
                        .entry(name)
                        .and_modify(|r| match r {
                            Ok(index) => {
                                let orig_fqn= fully_qualified_funcs
                                    .iter()
                                    .find(|(_, i)| **i == *index)
                                    .expect("we always push fully qualified paths along with short ones")
                                    .0
                                    .clone();
                                *r = Err(vec![orig_fqn, fqn.clone()]);
                            }
                            Err(ambiguities) => {
                                ambiguities.push(fqn.clone());
                            }
                        })
                        .or_insert(Ok(index));

                    fully_qualified_funcs.insert(fqn, index);
                }

                ComponentItem::Component(c) => {
                    for (name, item) in c.exports(engine) {
                        visit(
                            component,
                            engine,
                            item,
                            push_name(name),
                            fully_qualified_funcs,
                            short_funcs,
                        );
                    }
                }

                ComponentItem::ComponentInstance(c) => {
                    for (name, item) in c.exports(engine) {
                        visit(
                            component,
                            engine,
                            item,
                            push_name(name),
                            fully_qualified_funcs,
                            short_funcs,
                        );
                    }
                }

                _ => {}
            }
        }

        visit(
            component,
            engine,
            ComponentItem::Component(component.component_type()),
            Default::default(),
            &mut fully_qualified_funcs,
            &mut short_funcs,
        );

        (fully_qualified_funcs, short_funcs)
    }
}

struct HostState {
    ctx: WasiCtx,
    table: ResourceTable,
    stderr_writer: TracingStderrWriter,
}

impl HostState {
    fn flush_stderr(&mut self) {
        let _ = self.stderr_writer.flush();
    }
}

impl Default for HostState {
    fn default() -> Self {
        let stderr_writer = TracingStderrWriter::new();

        Self {
            // TODO(#2629): Preopen some directory if the runtime wants to.
            // TODO(#2625): Implement permissions system to allow users to limit these caps.
            ctx: WasiCtx::builder()
                .inherit_stdin()
                .inherit_stdout()
                .stderr(stderr_writer.clone())
                .allow_blocking_current_thread(true)
                .inherit_env()
                .inherit_network()
                .allow_ip_name_lookup(true)
                .build(),
            table: ResourceTable::new(),
            stderr_writer,
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
