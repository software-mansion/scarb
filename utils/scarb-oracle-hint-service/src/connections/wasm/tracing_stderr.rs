use std::io::{self, Write};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::io::AsyncWrite;
use tracing::{Span, debug, debug_span};
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
