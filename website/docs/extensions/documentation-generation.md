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

## Doc tests

`scarb doc` can extract and run code examples embedded in documentation comments, verifying that they compile and execute correctly. This helps ensure that code examples in your documentation stay up to date.

### Writing runnable examples

To make a code block runnable, add the `cairo` and `runnable` attributes to the code fence:

````cairo
/// Adds two numbers together.
/// ```cairo,runnable
/// let result = add(2, 3);
/// assert(result == 5, 'should be 5');
/// ```
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}
````

Code blocks marked only with `cairo` (without `runnable`) are not executed and will be ignored during testing.

### Function body vs full Cairo snippet

Scarb detects whether a code block contains a **function body** (expressions and statements) or a **full Cairo snippet** (with top-level items like functions) and handles each differently.

#### Function body

If the code block contains only expressions and statements, Scarb automatically wraps it in a `fn main() { ... }` and adds `use package_name::*;` so you can call items from the documented package directly:

````cairo
/// ```cairo,runnable
/// let result = add(2, 3);
/// println!("{}", result);
/// ```
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}
````

Under the hood this becomes:

```cairo
use hello_world::*;

#[executable]
fn main() {
    let result = add(2, 3);
    println!("{}", result);
}
```

#### Full Cairo snippet

If the code block contains top-level items (e.g. its own `fn main()`), Scarb uses it as-is without wrapping. You must define the entry point yourself:

````cairo
/// ```cairo,runnable
/// #[executable]
/// fn main() -> i32 {
///     add(-1, 1)
/// }
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
````

This form is useful when you need to return a value from `main`, define helper functions, or have full control over the example structure. The documented package is still imported with `use package_name::*;`.

### Code block attributes

Attributes are specified as a comma-separated list after the opening code fence:

| Attribute      | Description                                                                       |
| -------------- | --------------------------------------------------------------------------------- |
| `cairo`        | Marks the block as Cairo code.                                                    |
| `runnable`     | Marks the block for execution. Must be combined with `cairo`.                     |
| `ignore`       | Skips the block entirely (not compiled, not run).                                 |
| `no_run`       | Compiles the block but does not execute it.                                       |
| `compile_fail` | Asserts that the block **fails to compile**. The test passes on a compile error.  |
| `should_panic` | Asserts that the block **panics at runtime**. The test passes on a runtime error. |

Example using `should_panic`:

````cairo
/// ```cairo,runnable,should_panic
/// assert(is_odd(2), '2 is not odd');
/// ```
pub fn is_odd(n: i32) -> bool {
    n % 2 != 0
}
````

Example using `compile_fail`:

````cairo
/// ```cairo,runnable,compile_fail
/// is_odd(true);
/// ```
pub fn is_odd(n: i32) -> bool {
    n % 2 != 0
}
````

### Package import

The package import will only be added if the package declares a `[lib]` target.
Otherwise, it's not possible to import the package's code.
In such case, a warning will be shown and you need to provide all imports for your code manually.

### Running doc tests

Doc tests run automatically as part of `scarb doc`. When code blocks are found, each runnable block is compiled and executed in an isolated temporary workspace that depends on the documented package.

To skip running doc tests, use the `--no-run` flag:

```shell
scarb doc --no-run
```

The output shows the result for each code block:

```
   Running 3 doc examples for `hello_world`
test hello_world::bar ... ignored
test hello_world::foo ... ok
test hello_world::foo_bar ... ok

test result: ok. 2 passed; 0 failed; 1 ignored
```

When a code block has multiple examples, they are distinguished by index (e.g., `hello_world::add (example 0)`, `hello_world::add (example 1)`).

If any runnable example fails unexpectedly, `scarb doc` exits with a non-zero status.

### Embedding execution results

For Markdown output format, execution results of runnable examples are embedded directly into the generated documentation pages. Each successfully executed code block includes the captured output and return value below it.

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
