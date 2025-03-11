use crate::Message;

/// A message that prints a new line.
pub struct NewLine {}

impl Default for NewLine {
    fn default() -> Self {
        Self::new()
    }
}

impl NewLine {
    /// Create a new instance of `NewLine`.
    pub fn new() -> Self {
        Self {}
    }
}

impl Message for NewLine {
    fn print_text(self)
    where
        Self: Sized,
    {
        println!();
    }

    fn print_json(self)
    where
        Self: Sized,
    {
    }
}
