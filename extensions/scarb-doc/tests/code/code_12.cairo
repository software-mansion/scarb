  /// Function that prints "foo" to stdout with endline.
  /// Can invoke it like that:
  /// ```runnable
  /// foo();
  /// ```
  pub fn foo() {
      println!("foo");
  }

  /// Function that prints "bar" to stdout with endline.
  /// Can invoke it like that:
  /// ```cairo
  ///     bar();
  /// ```
  pub fn bar() {
      println!("bar");
  }

  /// Function that calls both foo and bar functions.
  /// Can invoke it like that:
  /// ```cairo,runnable
  /// foo_bar();
  /// ```
  pub fn foo_bar() {
      foo();
      bar();
  }


  /// Main function that cairo runs as a binary entrypoint.
  fn main() {
      println!("hello_world");
  }
