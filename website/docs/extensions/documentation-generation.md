# Generating Documentation

`scarb doc` is a tool that generates documentation based on the code comments. Generation supports different out formats. The result is being placed inside the `/target/doc`.

## Supported output formats

- Markdown. Fully supported by [mdBook](https://rust-lang.github.io/mdBook/). (Default)
- Custom JSON

## Available type of comments

As for now, only the `///` comment prefix is supported.

## mdBook

Generated markdown can be used to create documentation book.
Requirements:

- Install [mdBook](https://rust-lang.github.io/mdBook/guide/installation.html) running `cargo install mdbook`.
- Run `scarb doc` inside the project root.
- Run `mdbook build` (or `mdbook serve`) inside the generated documentation target (`/target/doc/<PACKAGE-NAME>`).

## Examples

Let's take for a example simple Cairo project initalized using `scarb new`. The code inside `lib.cairo`:

````cairo
//! Sub-package code (without feature)

/// Function that prints "test" to stdout with endline.
/// Can invoke it like that:
/// ```cairo
///     fn main() {
///         test();
///     }
/// ```
fn test() {
    println!("test");
}

/// Main function that cairo runs as a binary entrypoint.
fn main() {
    println!("hello_world");
}
````

After running `scarb doc`, inside the target directory, we should get:

- The `src` directory, which contains all the different markdown files.
- The `book.toml` which contains everything needed by [mdBook](https://rust-lang.github.io/mdBook/) to build documentation book.

Running `scarb doc --output-format json` will result in a single JSON file inside target directory.
