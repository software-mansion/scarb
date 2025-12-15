# Generating documentation

`scarb doc` is a tool that generates documentation based on the code comments. Generation supports different output formats. The result is being placed inside the `/target/doc` directory.

### Generating workspace documentation

You can run `scarb doc --workspace` to generate documentation for all packages in the workspace.

Use `--exclude` to omit the workspace packages in the documentation. Must be used with the `--workspace` flag.
Example:

```sh
scarb doc --workspace --exclude='package_name1, package_name2'
```

## Supported output formats

- Markdown. Fully supported by [mdBook](https://rust-lang.github.io/mdBook/). (Default)
- Custom JSON

## Available types of comments

As for now, we support those types of comments:

- `///` documentation for the following item.
- `//!` documentation for enclosing item (also works with file modules).

the `///` and `//!` comment prefixes are supported.

## Item linkage

You can also link to another item's page by just referring to the item within the documentation comment.
Currently, we support only those types of links:

- `[ItemName]` and ``[`ItemName`]`` (where `ItemName` is a valid path to an item).

## Linking to the source code GitHub repository

Use flag `--remote-base-url` or set `SCARB_DOC_REMOTE_BASE_URL` environmental variable to link to the source code of the package in the GitHub repository. Each item subpage will be linked to the corresponding source code file.
For example:

```shell
scarb doc --remote-base-url=https://github.com/ExampleRepoOwner/ExampleRepoProject/blob/example_branch/
```

Scarb constructs urls combining: the remote base url, the relative path to the file from the package root, and line anchors when relevant. It does not check whether the links are valid or if they were resolved correctly. It is up to the user to provide a correct base url and verify the results.
Example url for an item in a workspace package named `hello_world` with a relative path `src/lib.cairo`, line anchor start `10`, end `15`, and `SCARB_DOC_REMOTE_BASE_URL` like above, will be:

```
https://github.com/ExampleRepoOwner/ExampleRepoProject/blob/example_branch/hello_world/src/lib.cairo#L10-L15
```

Note, it's relevant for Markdown output only.

## mdBook

Generated Markdown can be used to build a [mdBook](https://rust-lang.github.io/mdBook) documentation.
You can do this directly from Scarb by running `scarb doc` with `--build` argument.

Alternatively, you can do this manually with the following steps:

- Install [mdBook](https://rust-lang.github.io/mdBook/guide/installation.html) by running `cargo install mdbook`.
- Run `scarb doc` inside the project root.
- Run `mdbook build` (or `mdbook serve`) inside the generated documentation target (`/target/doc/<PACKAGE-NAME>`).
  By default, mdBook generated documentation doesn't support Cairo code highlighting. To make it work, just replace the generated `book/highlight.js` with [this](https://github.com/software-mansion/scarb/tree/main/extensions/scarb-mdbook/theme) one.

### Supported mdbook syntax

- Code blocks
- Inline code
- Tables
- Links
- Lists
- Headings
- Bold and italic
- Rules
- Strikethrough
- File embedding

## Examples

Let's take, for example, a simple Cairo project initialized using `scarb new`. Let's change the code inside `lib.cairo` to:

````cairo
//! This module is an example one.
//! It tries to show how documentation comments work.


/// Example Enum. It's really similar to [ExampleStruct]
pub enum ExampleEnum {
    /// First enum variant.
    VARIANT_A,
    /// Second enum variant.
    VARIANT_B
}

/// Example struct. Contains a public field and a private one.
pub struct ExampleStruct {
    /// Private field.
    field_a: felt252,
    /// Public field.
    pub field_b: felt252,
    /// [`ExampleEnum`] field
    field_c: ExampleEnum
}

/// Function that prints "test" to stdout with endline.
/// Can invoke it like that:
/// ```cairo
///     fn main() {
///         test();
///     }
/// ```
pub fn test() {
    println!("test");
}

/// Main function that Cairo runs as a binary entrypoint.
/// This function uses [test] function.
pub fn main() {
    //! This is an inner comment. It refers to it's parent which is the main function.
    println!("hello_world");
    test();
}
````

After running `scarb doc`, inside the target directory, you will see the generated documentation in `mdBook` format which consists of:

- The `src` directory, which contains the contents of your book in files with Markdown format.
- The `book.toml` which contains settings for describing how to build your book.

Running `scarb doc --output-format json` will result in a single JSON file inside the target directory with collected documentation inside.
