use std::future::Future;

use crate::core::config::GetConfig;

pub fn block_on<F: Future>(config: impl GetConfig, future: F) -> F::Output {
    config.config().tokio_handle().block_on(future)
}
