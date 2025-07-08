# Hosting a registry

Although Scarb uses the official [scarbs.xyz](https://scarbs.xyz) registry by default, we provide you with the option to host your own.
You just need to implement an interface imposed by Scarb.

## Basic package registry

The most basic registry needs to serve three types of files over HTTP: index file, package index info and package archives.
That means a simple file server can be used as a package registry.

### Registry index file

A file that contains all the info about various endpoints of the registry that Scarb needs to be able to operate over.

The index file has a defined structure:

```json
{
  "version": 1,
  "api": "https://your.domain.com/api/v1",
  "dl": "https://your.domain.com/api/v1/dl/{package}/{version}",
  "index": "https://your.domain.com/api/v1/index/{prefix}/{package}.json"
}
```

- `version` - value reserved for versioning the registry interface.
- `api` - a URL of your registry API.
  Currently, isn't used at all, will be used for things like uploading or yanking a package.
- `dl` - a download template URL.
  You can use `{package}` and `{version}` to construct a template that Scarb will populate with the name and version of the package that it needs to download.
  In case of a simple server it could look like `https://your.registry.com/{package}-{version}.tar.zst`.
  The request to that URL must return a package `.tar.zst` archive created by the `scarb package` command.
- `index` - a URL template that functions like the one in `dl` but points to JSON files with index info about specific packages.
  It takes a `{package}` parameter, which is the package name, and a `{prefix}` value.

  This prefix is useful when you want to organize a file structure into a deeper but narrower one.
  This is the structure `scarb publish` creates when you use it with a local registry.
  Essentially, for a package with a name of 4 characters or longer (e.g. `foobar`), the prefix will be `/fo/ob/`.
  For 3 characters, it will be `/3/f`, and for 2 or 1 character(s), just `/2` and `/1`, respectively.

### Package index file

When Scarb needs to figure out which version of a package it needs to fetch in order to resolve dependency requirements, it needs to know what versions are available.
That's what package index files are used for.
They contain information about package versions present in the registry, along with data about their dependencies and checksums needed to verify that the package hasn't been tampered with.

A structure of an example `foo` package index file looks like this:

```json
[
  {
    "v": "0.1.0",
    "deps": [],
    "cksum": "sha256:6607a3b860f35f55738360ff55917642282d772423e8120a013b479ddb9e3f89"
  },
  {
    "v": "0.1.1",
    "deps": [
      {
        "name": "bar",
        "req": "^0.1.3"
      }
    ],
    "cksum": "sha256:5917642282d772423e8120a013b4796607a3b860f35f55738360ff5ddb9e3f89",
    "yanked": true
  }
]
```

As you can see, it is a JSON array with each entry representing a version of the `foo` package available in the registry, consisting as follows:

- `v` - key that describes the version.
- `deps` - array describing each version's dependency requirements.
- `cksum` - a `sha256` hash used to verify integrity.
- `yanked` - optional boolean indicating if this version has been deprecated (yanked), and should no longer be used for new installations.

### Package archive

The last type of files that needs to be served are the package archives.
These are the outputs of the `scarb package` command, as described in the [Packaging](./publishing) section.

## Using custom registry

To use a custom registry to download a specific dependency, you need to add a `registry` key to the entry.
It needs to point to a URL that returns the registry index file.

```toml
foo = { version = "0.1.3", registry = "https://custom.registry/index" }
```
