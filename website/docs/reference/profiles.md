# Profiles

Profiles provide a way to alter the compiler settings.

Scarb has 2 built-in profiles: `dev` and `release`.
The profile defaults to `dev` if a profile is not specified on the command-line.
In addition to the built-in profiles, custom user-defined profiles can also be specified.

Profile settings can be changed in Scarb.toml with the `[profile]` table.
Profile settings defined in dependencies will be ignored.

Currently, profiles define properties that affect the compiler settings, in a `[cairo]` table
(analogue to the [cairo](./manifest#cairo) section of the manifest definition) and custom tool metadata
(analogue to the [tool](./manifest#tool) section of the manifest definition).

## Overriding built-in profile properties

Each of the built-in profiles come with predefined default properties.

The properties of a built-in profile can be overridden by specifying a new property value in a custom profile.

For example, the `dev` profile has the `sierra-replace-ids` property set to `true` by default.
This can be overridden by specifying the same property in a custom profile:

```toml
[profile.dev.cairo]
# Replace all names in generated Sierra code with dummy counterparts.
sierra-replace-ids = true
```

## Defining custom profiles

Custom profiles can be defined in Scarb.toml with the `[profile]` table.
Each profile is defined by a name and a set of properties.

For example, the following defines a custom profile named `my-profile`:

```toml
[profile.my-profile]
```

A custom profile can be used with `--profile` argument. For instance:

```shell
scarb --profile my-profile build
```

### Profile inheritance

Each custom profile inherits the properties of one of the built-in profiles.
The built-in profile to inherit from is specified with the `inherits` property.

For example:

```toml
[profile.my-profile]
inherits = "release"
```

If not specified, the `dev` profile is used by default.
A custom profile can override properties of the inherited profile, analogous to how built-in profile properties can be
overridden.
