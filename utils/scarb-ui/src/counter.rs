use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub struct DiagnosticsCounter {
    pub errors: AtomicUsize,
    pub warnings: AtomicUsize,
}

impl DiagnosticsCounter {
    pub fn error(&self) {
        let _ = self.errors.fetch_add(1, Ordering::Release);
    }
    pub fn warning(&self) {
        let _ = self.warnings.fetch_add(1, Ordering::Release);
    }
    pub fn finish(&self) -> DiagnosticsCount {
        DiagnosticsCount {
            errors: self.errors.load(Ordering::Acquire),
            warnings: self.warnings.load(Ordering::Acquire),
        }
    }
}

pub struct DiagnosticsCount {
    pub errors: usize,
    pub warnings: usize,
}
