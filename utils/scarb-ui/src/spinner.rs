use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

use crate::Widget;

pub struct Spinner {
    message: String,
}

impl Spinner {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    fn default_style() -> ProgressStyle {
        ProgressStyle::with_template("{spinner:.cyan} {wide_msg} {elapsed}").unwrap()
    }
}

pub struct SpinnerHandle {
    pb: ProgressBar,
}

impl Drop for SpinnerHandle {
    fn drop(&mut self) {
        self.pb.finish_and_clear()
    }
}

impl Widget for Spinner {
    type Handle = SpinnerHandle;

    fn text(self) -> Self::Handle {
        let pb = ProgressBar::new_spinner()
            .with_style(Spinner::default_style())
            .with_message(self.message);
        pb.enable_steady_tick(Duration::from_millis(120));
        SpinnerHandle { pb }
    }
}
