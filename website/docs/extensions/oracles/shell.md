# `shell` protocol <Badge type="warning" text="experimental" />

The `shell` oracle protocol is a lightweight, built‑in protocol that executes a shell command and returns its standard
output to Cairo. Standard error is forwarded to the executor logs. It is useful for calling utility scripts available in
the running CLI environment; for example, in tests.

The `shell` library provides a type-safe interface for this protocol. Invoking this protocol directly is not
recommended.

<BigLink href="https://scarbs.xyz/packages/shell">
  Go to shell on scarbs.xyz
</BigLink>

## Connection string format

```
shell:
```

The connection string doesn't carry any parameters. The command to execute is provided as the first and only calldata
argument (a Cairo `ByteArray`).

#### Example

```cairo
oracle::invoke("shell:", "exec", "curl -sSLf https://api.github.com/zen");
```

## Execution model

- One‑shot execution per call. Subprocesses aren't kept alive between calls.
- The command line is parsed and executed by a minimal cross-platform shell, the same that
  powers [deno tasks](https://docs.deno.com/runtime/reference/cli/task/#syntax).
- The current process environment and working directory are inherited.
- Standard output is captured and returned to Cairo; standard error is streamed into executor's logs at debug level.

## Selectors

The `shell` protocol supports one selector called `exec`.
It waits for the command to finish and returns a tuple `(exit_code: i32, stdout: ByteArray)`.
