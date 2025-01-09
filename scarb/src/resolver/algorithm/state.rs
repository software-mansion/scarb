use crate::core::registry::Registry;
use crate::core::{ManifestDependency, Summary};
use crate::resolver::algorithm::in_memory_index::{InMemoryIndex, VersionsResponse};
use crate::resolver::algorithm::provider::DependencyProviderError;
use futures::{FutureExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

#[derive(Default)]
pub(crate) struct ResolverState {
    pub(crate) index: InMemoryIndex,
}

impl ResolverState {
    pub(crate) async fn fetch(
        self: Arc<Self>,
        registry: &dyn Registry,
        request_stream: mpsc::Receiver<Request>,
    ) -> Result<(), DependencyProviderError> {
        let mut response_stream = ReceiverStream::new(request_stream)
            .map(|request| self.process_request(request, registry).boxed_local())
            // Allow as many futures as possible to start in the background.
            // Backpressure is provided by at a more granular level by `packages` once map, as well as the bounded request channel.
            .buffer_unordered(usize::MAX);

        while let Some(response) = response_stream.next().await {
            match response? {
                Some(Response::Package(package, summaries)) => {
                    self.index
                        .packages()
                        .done(package, Arc::new(VersionsResponse::Found(summaries)));
                }
                None => {}
            }
        }
        Ok(())
    }

    async fn process_request(
        &self,
        request: Request,
        registry: &dyn Registry,
    ) -> Result<Option<Response>, DependencyProviderError> {
        match request {
            Request::Package(dependency) => {
                self.index.packages().register(dependency.clone());
                let summaries = registry.query(&dependency).await?;
                Ok(Some(Response::Package(dependency, summaries)))
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum Request {
    Package(ManifestDependency),
}

pub(crate) enum Response {
    Package(ManifestDependency, Vec<Summary>),
}
