use std::io::{self, Write};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::io::AsyncWrite;
use tracing::{Span, debug, debug_span, warn};
use wasmtime_wasi::cli::{IsTerminal, StdoutStream};

/// A custom stderr writer that forwards output to tracing logs in real-time.
#[derive(Clone)]
pub struct TracingStderrWriter {
    buffer: Arc<Mutex<Vec<u8>>>,
    span: Span,
}

impl TracingStderrWriter {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            span: debug_span!("err"),
        }
    }
}

impl Write for TracingStderrWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let _span = self.span.enter();

        let mut buffer = self.buffer.lock().unwrap();
        buffer.extend_from_slice(buf);

        // Find complete lines and process them using the lines pattern
        while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
            let line_with_newline = buffer.drain(..=newline_pos).collect::<Vec<u8>>();
            let line_bytes = &line_with_newline[..line_with_newline.len() - 1]; // Remove newline

            // Process line similar to BufRead::lines()
            match std::str::from_utf8(line_bytes) {
                Ok(line) => debug!("{}", line),
                Err(err) => warn!("{:?}", err),
            }
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let _span = self.span.enter();

        let mut buffer = self.buffer.lock().unwrap();
        if !buffer.is_empty() {
            // Handle final line that doesn't end with newline
            match std::str::from_utf8(&buffer) {
                Ok(line) => debug!("{}", line),
                Err(err) => warn!("{:?}", err),
            }
            buffer.clear();
        }
        Ok(())
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
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(self.write(buf))
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(self.flush())
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}
