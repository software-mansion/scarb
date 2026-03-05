mod inner {
    /// A struct defined in a submodule, with a runnable example.
    /// ```cairo, runnable
    /// println!("hello from re-exported item");
    /// ```
    pub struct MyStruct {}
}

pub use inner::MyStruct;
