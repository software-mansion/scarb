# Hosting a registry

Although official registry is in development, we want you to have an option to host your own.
You just need to implement an interface imposed by Scarb.

## Basic package registry

The most basic registry needs to serve three types of files over HTTP: index file, package index info and package archives.
That means a simple file server can be used as a package registry.

### Registry index file

A file that contains all the info about various endpoints of the registry that Scarb needs to be able to operate over.

Index file has a defined structure:

```json
{
  "version": 1,
  "api": "https://your.domain.com/api/v1",
  "dl": "https://your.domain.com/api/v1/dl/{package}/{version}",
  "index": "https://your.domain.com/api/v1/index/{prefix}/{package}.json"
}
```

- `version` - value reserved for versioning the registry interface.
- `api` - a URL of your registry API. Currently, isn't used at all, will be used for things like uploading or yanking a package.
- `dl` - a download template URL.
  You can use `{package}` and `{version}` to construct a template that Scarb will populate with the name and version of the package that it needs to download.
  In case of a simple server it could look like `https://your.registry.com/{package}-{version}.tar.zst`.
  The request to that URL must return a package `.tar.zst` archive created by `scarb package` command.
- `index` - a URL template that functions like the one in `dl` but it points to JSON files with index info about specific packages.
  It takes `{package}` parameter which is a package name but also a `{prefix}` value.

  This prefix is useful when you want to organize a file structure into a deeper but narrower one.
  That's the structure `scarb publish` creates when you use it with local registry.
  Basically for a package that has name of 4 characters or longer (e.g. foobar) the prefix will be `/fo/ob/`, for 3 characters `/3/f` and for 2 and 1 characters just `/2` and `/1` respectively.

### Package index file

When Scarb needs to figure out what version of package it needs to fetch in order to resolve dependency requirements it needs to know what versions are available.
That's what package index files are used for.
They contain information about package versions present in the registry together with data about what dependencies they have and checksums needed to verify if package haven't been tampered with.

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
    "cksum": "sha256:5917642282d772423e8120a013b4796607a3b860f35f55738360ff5ddb9e3f89"
  }
]
```

As you can see it is a JSON array with each entry being a version of the `foo` package available in the registry.
An entry consist of `v` key that describes the version, `deps` array describing each version dependency requirements and `cksum` which is an `sha256` hash used to verify integrity.

### Package archive

Last type of files that needs to be served are the package archives.
These are the output of `scarb package` command as described in [Packaging](./packaging) section.

## Using custom registry

To use a custom registry to download specific dependency you need to add `registry` key to the entry.
It needs to point to a URL that returns the registry index file.

```toml
foo = { version = "0.1.3", registry = "https://custom.registry/index" }
```
