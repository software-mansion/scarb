# Generating documentation

`scarb doc` is a tool that generates documentation based on the code comments. Generation supports different output formats. The result is being placed inside the `/target/doc`.

## Supported output formats

- Markdown. Fully supported by [mdBook](https://rust-lang.github.io/mdBook/). (Default)
- Custom JSON

## Available types of comments

As for now, only the `///` comment prefix is supported.

## mdBook

Generated markdown can be used to create a documentation book.
Requirements:

- Install [mdBook](https://rust-lang.github.io/mdBook/guide/installation.html) by running `cargo install mdbook`.
- Run `scarb doc` inside the project root.
- Run `mdbook build` (or `mdbook serve`) inside the generated documentation target (`/target/doc/<PACKAGE-NAME>`).

## Examples

Let's take, for example, a simple Cairo project initalized using `scarb new`. Let's change the code inside `lib.cairo` to:

````cairo
/// Example Enum.
enum ExampleEnum {
    /// First enum variant.
    VARIANT_A,
    /// Second enum variant.
    VARIANT_B
}

/// Example struct. Contains a public field and a private one.
struct ExampleStruct {
    /// Private field.
    field_a: felt252,
    /// Public field.
    pub field_b: felt252
}

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

/// Main function that Cairo runs as a binary entrypoint.
fn main() {
    println!("hello_world");
}
````

After running `scarb doc`, inside the target directory, you will see the generated documentation in `mdBook` format which consists of:

- The `src` directory, which contains the contents of your book in files with markdown format.
- The `book.toml` which contains contains settings for describing how to build your book.

Running `scarb doc --output-format json` will result in a single JSON file inside the target directory with collected documentation inside.

## Cairo code highlighting using mdBook

By default, mdBook generated documentation doesn't support Cairo code highlighting. To make it work, just replace the generated `book/highlight.js` with [this](https://github.com/software-mansion/scarb/tree/main/extensions/scarb-doc/theme) one.
