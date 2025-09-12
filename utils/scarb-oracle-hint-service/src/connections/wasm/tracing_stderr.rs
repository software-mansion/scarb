/// A custom stderr writer that forwards output to tracing logs in real-time.
#[derive(Clone)]
pub struct TracingStderrWriter {
    buffer: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    span: tracing::Span,
}

impl TracingStderrWriter {
    pub fn new() -> Self {
        Self {
            buffer: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            span: tracing::debug_span!("err"),
        }
    }
}

impl std::io::Write for TracingStderrWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let _span = self.span.enter();

        let mut buffer = self.buffer.lock().unwrap();
        buffer.extend_from_slice(buf);

        // Process complete lines immediately
        let mut start = 0;
        while let Some(end) = buffer[start..].iter().position(|&b| b == b'\n') {
            let line_end = start + end;
            if let Ok(line) = std::str::from_utf8(&buffer[start..line_end]) {
                tracing::debug!("{}", line);
            }
            start = line_end + 1;
        }

        // Keep remaining incomplete line in buffer
        if start > 0 {
            buffer.drain(0..start);
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let _span = self.span.enter();

        let mut buffer = self.buffer.lock().unwrap();
        if !buffer.is_empty() {
            if let Ok(text) = std::str::from_utf8(&buffer) {
                tracing::debug!("{}", text);
            }
            buffer.clear();
        }
        Ok(())
    }
}

impl wasmtime_wasi::cli::IsTerminal for TracingStderrWriter {
    fn is_terminal(&self) -> bool {
        false
    }
}

impl wasmtime_wasi::cli::StdoutStream for TracingStderrWriter {
    fn async_stream(&self) -> Box<dyn tokio::io::AsyncWrite + Send + Sync> {
        Box::new(self.clone())
    }
}

impl tokio::io::AsyncWrite for TracingStderrWriter {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        std::task::Poll::Ready(std::io::Write::write(&mut *self, buf))
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(std::io::Write::flush(&mut *self))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        self.poll_flush(cx)
    }
}
