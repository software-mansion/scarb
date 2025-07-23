use crate::compiler::helpers::{write_json, write_string};
use crate::core::Workspace;
use crate::flock::Filesystem;
use anyhow::{Context, Result};
use cairo_lang_sierra::program::{ProgramArtifact, VersionedProgram};
use std::marker::PhantomData;
use std::sync::{Arc, mpsc};
use std::{mem, thread};

pub type ArtifactsWriterRequestSink = mpsc::Sender<Request>;
pub type ArtifactsWriterRequestStream = mpsc::Receiver<Request>;

pub enum Request {
    ProgramArtifact {
        file: File,
        value: Arc<ProgramArtifact>,
    },
    ProgramArtifactText {
        file: File,
        value: Arc<ProgramArtifact>,
    },
}

pub struct File {
    pub file_name: String,
    pub description: String,
    pub target_dir: Filesystem,
}

pub struct ArtifactsWriter<'a> {
    handle: Option<thread::JoinHandle<Result<()>>>,
    _phantom_data: PhantomData<Workspace<'a>>,
}

impl<'a> ArtifactsWriter<'a> {
    pub fn new(request_stream: ArtifactsWriterRequestStream, ws: &Workspace<'a>) -> Self {
        let ws = unsafe {
            // This should be safe, as we know we will join the artifact writer thread before
            // the workspace is dropped.
            mem::transmute::<&Workspace<'_>, &Workspace<'_>>(ws)
        };
        let handle = thread::Builder::new()
            .name("scarb-artifacts-writer".into())
            .spawn(move || {
                for request in request_stream.iter() {
                    handle_request(request, ws)
                        .with_context(|| "failed to handle artifact writer request")?;
                }
                Ok(())
            })
            .expect("failed to spawn artifacts writer thread");
        Self {
            handle: Some(handle),
            _phantom_data: PhantomData,
        }
    }

    pub fn join(mut self) -> Result<()> {
        let result = match self.handle.take() {
            Some(handle) => handle
                .join()
                .expect("failed to join artifacts writer thread"),
            None => Ok(()),
        };
        mem::forget(self); // Defuse the drop bomb.
        result
    }
}

impl<'a> Drop for ArtifactsWriter<'a> {
    fn drop(&mut self) {
        panic!("not defused: ArtifactsWriter dropped without join");
    }
}

#[tracing::instrument(level = "trace", skip_all)]
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
            // Cloning the underlying program is expensive, but we can afford it here,
            // as we are on a dedicated thread anyway.
            let sierra_program: VersionedProgram = value.as_ref().clone().into();
            write_json(
                file_name.as_str(),
                description.as_str(),
                &target_dir,
                ws,
                &sierra_program,
            )?;
        }
        Request::ProgramArtifactText {
            file:
                File {
                    file_name,
                    description,
                    target_dir,
                },
            value,
        } => {
            // vide supra
            let sierra_program: VersionedProgram = value.as_ref().clone().into();
            write_string(
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
