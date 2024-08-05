<script setup>
import { data as rel } from "./github.data";
import AssetsTable from "./.vitepress/components/AssetsTable.vue";
</script>

# Download and install

[[toc]]

Scarb is a native command line application that is available for Linux, macOS and Windows on all common CPU
architectures.

Scarb follows a biweekly release schedule.
Each release may include new features, enhancements, bug fixes, deprecations and breaking changes.
For detailed information about each release, consult
the [release notes](https://github.com/software-mansion/scarb/releases).

If you are not sure if you have Scarb installed or not, you can run `scarb --version` in your terminal.

## Requirements

To download Git dependencies, Scarb requires a Git executable to be available in the `PATH` environment variable.

## Install via installation script

Installing via installation script is the fastest way to get Scarb up and running.
This method only works on macOS and Linux.

Run the following in your terminal, then follow the onscreen instructions.
This will install the latest **stable** release.

```shell
curl --proto '=https' --tlsv1.2 -sSf https://docs.swmansion.com/scarb/install.sh | sh
```

Run following command if you want to install the latest **nightly** release.

```shell
curl --proto '=https' --tlsv1.2 -sSf https://docs.swmansion.com/scarb/install.sh | sh -s -- -v nightly
```

If you want to install a specific version of Scarb (such as a preview or nightly version), run the following with the desired
version number.

```shell-vue
curl --proto '=https' --tlsv1.2 -sSf https://docs.swmansion.com/scarb/install.sh | sh -s -- -v {{ rel.sampleVersion }}
```

### Uninstall

The installation script does not have uninstalling logic built-in.
It tries to minimize changes performed to the system, though, to keep the number of manual steps to remove Scarb low.

1. Remove the `$XDG_DATA_HOME/scarb-install` directory, usually this is `~/.local/share/scarb-install`.
2. Remove `~/.local/bin/scarb` symbolic link.

The installation script might have added path to `~/.local/bin` to `$PATH`, by adding appropriate lines
to `.bashrc`, `.zshrc` or others, depending on the shell running in the system.
If you wish, you can remove these lines, but often this is not desirable.

On top of that, Scarb creates several files (like data files or caches) in standard system paths.
These have to be removed manually.
The list of all global paths written to by Scarb is listed in [here](./docs/reference/global-directories).

## Install via asdf

asdf is a CLI tool that can manage multiple language runtime versions on a per-project basis.
Scarb team maintains an [official plugin](https://github.com/software-mansion/asdf-scarb) for asdf which manages
multiple Scarb installations.
Mind that asdf works on macOS and Linux only.
This plugin needs `bash`, `curl`, `tar` and other generic POSIX utilities.
Everything should be included by default on your system.

When you have asdf already [installed](https://asdf-vm.com/guide/getting-started.html),
run the following command to add the `scarb` plugin:

```shell
asdf plugin add scarb
```

Show all installable versions:

```shell
asdf list-all scarb
```

Install latest version:

```shell
asdf install scarb latest
```

Install specific version:

```shell-vue
asdf install scarb {{ rel.sampleVersion }}
```

Set a version globally (in your `~/.tool-versions` file):

```shell
asdf global scarb latest
```

Check [asdf guide](https://asdf-vm.com/guide/getting-started.html) for more instructions on how to install & manage
versions.

## By operating system

Choose your operating system and tool.

### Windows

As for now, Scarb on Windows needs manual installation, but necessary steps are kept to minimum:

1. [Download the release archive](/download#precompiled-packages) matching your CPU architecture.
2. Extract it to a location where you would like to have Scarb installed.
   A folder named `scarb` in
   your [`%LOCALAPPDATA%\Programs`](https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid?redirectedfrom=MSDN#FOLDERID_UserProgramFiles)
   directory will suffice:
   ```batch
   %LOCALAPPDATA%\Programs\scarb
   ```
3. Add path to the `scarb\bin` directory to your `PATH` environment variable.
4. Verify installation by running the following command in new terminal session, it should print Scarb and Cairo
   language versions:
   ```shell
   scarb --version
   ```

#### Uninstall

Simply undo steps done to manually install Scarb:

1. Remove extracted archive files.
2. Remove the path to the `scarb\bin` directory from `PATH`.

On top of that, Scarb creates several files (like data files or caches) in standard system paths.
These have to be removed manually.
The list of all global paths written to by Scarb is listed in [here](./docs/reference/global-directories).

### NixOS

The community-maintained Cairo Nix overlay provides a ready-to-use Cairo development environment, which includes Scarb.

```shell
nix shell github:cairo-nix/cairo-nix
```

<BigLink href="https://github.com/cairo-nix/cairo-nix">
   Go to cairo-nix on GitHub
</BigLink>

## Precompiled packages

### Stable version

The current stable version of Scarb is <code>{{ rel.stable.version }}</code>.

<p><AssetsTable :release="rel.stable" /></p>

### Preview version

<template v-if="rel.preview">
<p>The current preview version of Scarb is <code>{{ rel.preview.version }}</code>.</p>
<p><AssetsTable :release="rel.preview" /></p>
</template>
<template v-else>
<p>There is no preview version of Scarb currently.</p>
</template>

### Archived versions

For older releases, go to the [releases](https://github.com/software-mansion/scarb/releases) page in Scarb GitHub
repository.

## Nightly builds

Scarb team publishes nightly builds of Scarb several times a week in separate GitHub repository.
These builds are built on top of Scarb's and Cairo compiler's latest `main` branches.
Consult release notes for exact commit hashes and more information.
Each build is identified by calendar day it was produced, and are tagged in the following
pattern: `nightly-YYYY-MM-DD`, for example: `nightly-2023-08-03`.

These builds are created automatically, unattended.
**Use at your own risk.**

<BigLink href="https://github.com/software-mansion/scarb-nightlies/releases">
   Go to Scarb nightly releases on GitHub
</BigLink>

## Platform support

Scarb is officially supported on the following platforms:

- `aarch64-apple-darwin`
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `x86_64-pc-windows-msvc`
- `x86_64-unknown-linux-gnu`

The builds for following platforms builds are experimental and are not supported by Scarb team.
These builds may stop being published in the future.
Use at your own risk:

- `aarch64-unknown-linux-musl`
- `x86_64-unknown-linux-musl`

## Source code

Scarb is an open source project developed by [Software Mansion](https://swmansion.com), available under terms of the MIT
License.
We host our Git repository on GitHub.
We also welcome external contributors!

<BigLink href="https://github.com/software-mansion/scarb">
   Go to Scarb on GitHub
</BigLink>
