use crate::compiler::helpers::write_json;
use crate::core::Workspace;
use crate::flock::Filesystem;
use anyhow::{Context, Result};
use cairo_lang_sierra::program::{ProgramArtifact, VersionedProgram};
use std::sync::mpsc;
use std::{mem, thread};
use tracing::trace_span;

pub enum Request {
    ProgramArtifact {
        file: File,
        value: Box<ProgramArtifact>,
    },
}

pub struct File {
    pub file_name: String,
    pub description: String,
    pub target_dir: Filesystem,
}

pub struct ArtifactsWriter {
    handle: Option<thread::JoinHandle<Result<()>>>,
}

impl ArtifactsWriter {
    pub fn new(request_stream: mpsc::Receiver<Request>, ws: &Workspace<'_>) -> Self {
        let ws = unsafe {
            // This should be safe, as we know we will join the artifact writer thread before
            // the workspace is dropped.
            mem::transmute::<&Workspace<'_>, &Workspace<'_>>(ws)
        };
        let handle = thread::Builder::new()
            .name("scarb-artifacts-writer".into())
            .spawn(move || {
                let span = trace_span!("writer requests");
                for request in request_stream.iter() {
                    let _guard = span.enter();
                    handle_request(request, ws)
                        .with_context(|| "failed to handle artifact writer request")?;
                }
                Ok(())
            })
            .expect("failed to spawn artifacts writer thread");
        Self {
            handle: Some(handle),
        }
    }

    pub fn join(mut self) -> Result<()> {
        let result = if let Some(handle) = self.handle.take() {
            handle
                .join()
                .expect("failed to join artifacts writer thread")
        } else {
            Ok(())
        };
        mem::forget(self); // Defuse the drop bomb.
        result
    }
}

impl Drop for ArtifactsWriter {
    fn drop(&mut self) {
        panic!("not defused: ArtifactsWriter dropped without join");
    }
}

fn handle_request(request: Request, ws: &Workspace<'_>) -> Result<()> {
    match request {
        Request::ProgramArtifact {
            file:
                File {
                    file_name,
                    description,
                    target_dir,
                },
            value,
        } => {
            let sierra_program: VersionedProgram = (*value).into();
            write_json(
                file_name.as_str(),
                description.as_str(),
                &target_dir,
                ws,
                &sierra_program,
            )?;
        }
    }
    Ok(())
}
