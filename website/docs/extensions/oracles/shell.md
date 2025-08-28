# `shell` protocol <Badge type="warning" text="experimental" />

The `shell` oracle protocol is a lightweight, built‑in protocol that executes a shell command and returns its standard
output to Cairo. Standard error is forwarded to the executor logs. It is useful for calling utility scripts available in
running CLI environment; for example, in tests.

## Connection string format

```
shell:
```

The connection string doesn't carry any parameters. The command to execute is provided as the first and only calldata
argument (a Cairo `ByteArray`).

#### Example

```cairo
oracle::invoke("shell:", 'taskco', "curl -sSLf https://api.github.com/zen");
```

## Execution model

- One‑shot execution per call. Subprocesses aren't kept alive between calls.
- The command line is parsed and executed by a minimal cross-platform shell, the same that
  powers [deno tasks](https://docs.deno.com/runtime/reference/cli/task/#syntax).
- The current process environment and working directory are inherited.
- Standard output is captured and returned to Cairo; standard error is streamed into Scarb's logs at debug level.

## Selectors (modes)

The `shell` protocol supports several subprocess calling behaviours, determined by invoked selector. They're nearly
identical and differ only in how failures are reported and what is returned:

| Selector | Return type                                                          | Failure handling                                             |
|----------|----------------------------------------------------------------------|--------------------------------------------------------------|
| `taskeo` | <code style="text-wrap:nowrap">(code: i32, stdout: ByteArray)</code> | Never errors after subprocess spawns; exit code is returned. |
| `taskco` | `stdout: ByteArray`                                                  | Errors if exit code ≠ 0.                                     |
