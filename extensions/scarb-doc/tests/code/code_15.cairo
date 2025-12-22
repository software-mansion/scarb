  /// Function that returns the sum of two integers.
  /// Example 1:
  /// ```cairo, runnable
  /// let x = add(2, 3);
  /// println!("{}", x);
  /// ```
  /// Example 2:
  /// ```cairo, runnable
  /// fn main() -> i32 {
  ///     add(-1, 1)
  /// }
  /// ```
  pub fn add(a: i32, b: i32) -> i32 {
      a + b
  }

  /// Main function that cairo runs as a binary entrypoint.
  fn main() {
      println!("hello_world");
  }
