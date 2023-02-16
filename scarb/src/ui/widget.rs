/// A persistent message that is only usable for humans, for example a spinner.
pub trait Widget {
    /// Allows for live interaction with the widget, and its drop is called when the widget should
    /// be cleared.
    type Handle;

    fn text(self) -> Self::Handle;
}
