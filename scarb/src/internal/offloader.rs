use crate::core::Workspace;
use anyhow::{Context, Result};
use std::marker::PhantomData;
use std::sync::mpsc;
use std::thread::Scope;
use std::{mem, thread};
use tracing::debug;

pub type BoxedWorkload<'a> = Box<dyn FnOnce(&'a Workspace<'a>) -> Result<()> + Send>;
pub type OffloaderSink<'a> = mpsc::Sender<(&'static str, BoxedWorkload<'a>)>;
pub type OffloaderStream<'a> = mpsc::Receiver<(&'static str, BoxedWorkload<'a>)>;

pub struct Offloader<'a> {
    handle: Option<thread::ScopedJoinHandle<'a, Result<()>>>,
    _phantom_data: PhantomData<Workspace<'a>>,
    sink: Option<OffloaderSink<'a>>,
}

impl<'a> Offloader<'a> {
    pub fn new(scope: &'a Scope<'a, '_>, ws: &'a Workspace<'a>) -> Self {
        let ws = unsafe {
            // This should be safe, as we know we will join the artifact writer thread before
            // the workspace is dropped.
            mem::transmute::<&Workspace<'a>, &Workspace<'static>>(ws)
        };
        let (sink, stream): (OffloaderSink<'a>, OffloaderStream<'a>) = mpsc::channel();
        let handle = thread::Builder::new()
            .name("scarb-offloader".into())
            .spawn_scoped(scope, move || {
                for (what, workload) in stream.iter() {
                    handle_request(what, workload, ws)?;
                }
                anyhow::Ok(())
            })
            .expect("failed to spawn artifacts writer thread");
        Self {
            sink: Some(sink),
            handle: Some(handle),
            _phantom_data: PhantomData,
        }
    }

    pub fn join(mut self) -> Result<()> {
        if let Some(sink) = self.sink.take() {
            drop(sink); // Close the channel to signal the thread to finish.
        }
        let result = match self.handle.take() {
            Some(handle) => handle
                .join()
                .expect("failed to join artifacts writer thread"),
            None => Ok(()),
        };
        mem::forget(self); // Defuse the drop bomb.
        result
    }

    pub fn offload(
        &self,
        what: &'static str,
        workload: impl FnOnce(&Workspace<'_>) -> Result<()> + Send + 'static,
    ) {
        self.sink
            .as_ref()
            .expect("offloader not initialized")
            .send((what, Box::new(workload)))
            .expect("failed to send request to offloader");
    }
}

#[tracing::instrument(level = "trace", skip(ws, workload))]
fn handle_request<'a>(
    what: &str,
    workload: BoxedWorkload<'a>,
    ws: &'a Workspace<'a>,
) -> Result<()> {
    debug!("handling offloader request for `{what}`");
    workload(ws).with_context(|| format!("failed to handle offloader request for `{what}`"))?;
    Ok(())
}
