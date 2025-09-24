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
    // NOTE: This is AsyncWrite only because wasmtime takes such an interface for piping stdio.
    //   Our implementation is entirely blocking, we dump the output as it comes directly to logs
    //   and wait for them to be handled by subscribers. There are two questions one might ask:
    //
    //   Q: Could synchronous stderr logging slow down WASM execution?
    //   A: Theoretically, yes. Synchronous emission means the WASM host thread waits for logging.
    //      This is intentional to preserve execution-log correlation and immediate log visibility.
    //      But WASM is executed here as a sidecar to a zero-knowledge proving runtime. It is not
    //      expected for these programs to be realtime or emit large amounts of logs.
    //
    //   Q: Is chopping buffers by lines and treating trailing data as full line correct?
    //      Won't this cause some log lines to be split in half?
    //   A: Most runtimes (and primarily Rust, which matters the most for us) buffer stdio by full
    //      lines, so this naturally plays well. For others... does it actually matter? This is
    //      just a log stream, and thus information will be still transmitted fully in weird format.

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
