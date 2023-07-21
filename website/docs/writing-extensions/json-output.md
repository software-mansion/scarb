# JSON output

When passing `--json` argument, Scarb will output all messages in newline-delimited JSON format.
This is useful, when running Scarb from scripts or other programs.
This format is guaranteed to be more stable than human-readable output.

Example output for compilation:

```shell
$ scarb --json build
{"status":"compiling","message":"hello v0.1.0 ([..]Scarb.toml)"}
{"type":"diagnostic","message":"error: Skipped tokens. Expected: Module/Use/FreeFunction/ExternFunction/ExternType/Trait/Impl/Struct/Enum or an attribute./n --> lib.cairo:1:1/nnot_a_keyword/n^***********^/n/n"}
{"type":"error","message":"could not compile `hello` due to previous error"}
```

Scarb outputs all JSON messages as fast as possible.
It is fine to rely on message appearance times for computing timings of command execution.
