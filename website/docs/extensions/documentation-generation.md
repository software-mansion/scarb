# Generating Documentation

`scarb doc` is a tool that generates documentation based on the code comments. Generation supports different out formats. The result is being placed inside the `/target/doc`.

## Supported output formats
- Markdown. Fully supported by [mdBook](https://rust-lang.github.io/mdBook/).
- Custom JSON

## Available type of comments
As for now, only the `///` comment prefix is supported.

