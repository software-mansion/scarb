use std::future::IntoFuture;

pub trait AwaitSync {
    /// The output that the future will produce on completion.
    type Output;

    /// Synchronously await a future by starting a small one-off runtime internally.
    fn await_sync(self) -> Self::Output;
}

impl<F: IntoFuture> AwaitSync for F {
    type Output = F::Output;

    fn await_sync(self) -> Self::Output {
        smol::block_on(self.into_future())
    }
}
