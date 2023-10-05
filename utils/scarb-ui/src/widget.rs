/// A persistent message that is only usable for humans, for example a spinner.
pub trait Widget {
    /// Allows for live interaction with the widget, and its drop is called when the widget should
    /// be cleared.
    type Handle;

    /// Display the widget on the standard output, and return a handle for further interaction.
    fn text(self) -> Self::Handle;
}
