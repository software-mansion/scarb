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

The `///` and `//!` comment prefixes are supported.

## Item linkage

You can also link to another item's page by just referring to the item within the documentation comment.
Currently, we support only those types of links:

- `[ItemName]` and ``[`ItemName`]`` (where `ItemName` is a valid path to an item).

## Linking to the source code VCS repository

You can add "View source" links in generated docs that point to your VCS repository. There are two ways to enable linking:

#### 1. Use the package manifest [`repository`](../reference/manifest.md#repository) field

If your package manifest `Scarb.toml` specifies a [`repository`](../reference/manifest.md#repository) that points to a VCS project website, Scarb can compose links automatically. This requires running `scarb doc` command in a VCS repository that Scarb can discover. If Git repository discovery fails, remote linking is disabled for this package.

#### 2. Explicit base URL (CLI/env)

Provide a VCS project website via flag `--remote-base-url` or `SCARB_DOC_REMOTE_BASE_URL` environmental variable. Scarb will append the file path (relative to the workspace root) and, when available, a VCS line range anchor.

Example:

```shell
scarb doc --remote-base-url=https://github.com/ExampleRepoOwner/ExampleRepoProject/blob/example_branch/
```

If a workspace package named `hello_world` contains `src/lib.cairo` and an item spans lines 10–15, the link will look like:

```
https://github.com/ExampleRepoOwner/ExampleRepoProject/blob/example_branch/hello_world/src/lib.cairo#L10-L15
```

Note that Scarb does not validate that the link targets exist. You must provide a correct base URL and verify the results yourself.

Precedence and disabling:

- If both a manifest `repository` and `--remote-base-url` are provided, the `--remote-base-url` flag takes precedence for link generation.
- To turn off link generation, use `--disable-remote-linking`.

Output format constraints:

- Remote linking is supported for Markdown output only.

Requirements:

- Linking requires either a manifest `repository` or an explicit `--remote-base-url`. If neither is configured and linking is not disabled, Scarb will error.

Officially supported VCS providers:

- GitHub
- GitLab

Scarb does not automatically detect your repository host. Instead, it assumes a standard URL structure common to the providers listed above.

While other VCS hosts are not officially supported, they may still work if they follow the same URL formatting for file browsing and line anchors (e.g., using `/blob/` for file paths and `#L` for line ranges). We may add official support or configuration options for other providers in the future based on community demand.

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

After running `scarb doc` you will see the generated documentation in `mdBook` format inside the target directory. It consists of:

- The `src` directory, which contains the contents of your book in Markdown format.
- The `book.toml` file, which contains settings describing how to build your book.

Running `scarb doc --output-format json` will result in a single JSON file inside the target directory containing the collected documentation.
