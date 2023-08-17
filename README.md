# Scarb

Scarb is the project management tool for the [Cairo] language.
Scarb manages your dependencies, compiles your projects and works as an extensible platform assisting in development.

## Documentation

All information about Scarb is available on project's [website](https://docs.swmansion.com/scarb/).

* [Installation](https://docs.swmansion.com/scarb/download.html)
* [Cheat sheet](https://docs.swmansion.com/scarb/docs/cheatsheet.html)
* [Documentation](https://docs.swmansion.com/scarb/docs.html)

## Changelog

All notable changes to this project are documented on the [GitHub releases] page.

## Roadmap

Scarb is under active development! Expect a lot of new features to appear soon! 🔥

- [x] Building Cairo packages
- [x] Pulling dependencies from local filesystem
- [x] Pulling dependencies from Git
- [x] Machine-readable workspace metadata generation
- [x] Built-in Cairo compiler plugins as packages (the `starknet` package)
- [x] Feature parity with Cairo compiler CLIs
- [x] Scarb installer
- [x] ASDF plugin
- [x] GitHub action
- [x] Workspaces
- [x] Nightlies
- [ ] Standardized `test` target
- [ ] `scarb init` templates or `scarb create-starknet-contract` command
- [ ] `Scarb.lock`
- [ ] Package registry
- [ ] PubGrub implementation for version resolution
- [ ] `scarb update`
- [ ] Dynamic loading of custom Cairo compiler plugins as Scarb packages
- [ ] Dynamic loading of custom targets aka code generators
- [ ] `scarb check`
- [ ] Dependency overrides
- [ ] Signing & notarization of Windows & macOS binaries

## Credits

This product includes modified portions of code of [Cargo], developed by the Rust project contributors.

This product includes modified portions of code of [hex_solver], developed by Six Colors AB.

[Cairo]: https://www.cairo-lang.org/

[Cargo]: https://github.com/rust-lang/cargo

[github releases]: https://github.com/software-mansion/scarb/releases

[hex_solver]: https://github.com/hexpm/hex_solver
