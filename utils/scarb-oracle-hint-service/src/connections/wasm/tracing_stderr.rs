use std::io::{self, BufRead};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncWrite;
use tracing::{Span, debug, debug_span, warn};
use wasmtime_wasi::cli::{IsTerminal, StdoutStream};

/// A custom stderr writer that forwards output to tracing logs in real-time.
#[derive(Clone)]
pub struct TracingStderrWriter {
    span: Span,
}

impl TracingStderrWriter {
    pub fn new() -> Self {
        Self {
            span: debug_span!("err"),
        }
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
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let _span = self.span.enter();
        for line in buf.lines() {
            match line {
                Ok(line) => debug!("{line}"),
                Err(err) => warn!("{err:?}"),
            }
        }
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}
