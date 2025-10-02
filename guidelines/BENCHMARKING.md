# Manually benchmarking Scarb

If you suspect your changes may affect performance, it's crucial to benchmark them to ensure no regressions occur.
However, benchmarking Scarb in a reliable and reproducible way is not a trivial thing to do. 
This guide should provide tools, guidelines and best practices to help you with that.

### Build configuration

**Always compile Scarb in release mode for benchmarking!**

The codegen optimizations performed by the Rust compiler will drastically affect benchmark results.
In the default, dev, build profile the optimizations are disabled.
To compile Scarb in release mode, use the following command:

```sh
cargo build --release -p scarb
```

### Recommended tools

It's a good idea to use [`hyperfine`](https://github.com/sharkdp/hyperfine) or a similar tool to run the benchmarks. 
This way you can easily run the benchmarks multiple times and get a more reliable result.
Hyperfine will calculate the mean, standard deviation and other statistics for you automatically!
It will also warn you if outliers occur between the runs. 
You can use `--warmup` flag to specify how many runs should happen before the benchmarking starts.
You can also use `--runs` flag to specify how many runs should be performed.

For instance, your hyperfine command could look like this:
```sh
hyperfine --warmup 1 --runs 10 -- 'scarb build -w'
```

### Warmup runs and Scarb configuration

If you run Scarb for the first time on some project, it will usually take longer than later runs.
This is because Scarb will first download all the dependencies of your project and save them to a global cache directory.
Additionally, the later runs can use incremental compilation to speed up the build process.
To stop this characteristic from skewing your results, run Scarb once before starting the benchmarking.
There is generally no need to run more than one warmup run.

If you are testing the performance of compilation, you may want to stop Scarb from loading the incremental caches. 
To do this, you can set an env variable `SCARB_INCREMENTAL=false` before running the benchmark.
Frequent times, it may be worth performing two benchmark rounds - one with incremental compilation and one without.
We care for the performance of both!

### Choosing a propper benchmarking input

Choosing the right Cairo project to benchmark on is a bit tricky. 
Usually, Scarb is fast enough that trivially small projects will compile so fast that changes to the compilation time 
will not be statistically significant. 
Specific characteristics of your project may also affect the results. 

It may be worth asking yourself questions like these:
- Does the project have a lot of remote dependencies?
- Does the project consist of one big package vs. many small packages?
- Does this project contain any Cairo contracts?
- How many compilation units are there?
- Does this project contain any Cairo tests? Are those integration or unit kind?
- Does this project compile with inlining, or without? 
- Does this project specify any `build-external-contracts` flags?
- Does this project use any Scarb plugins / proc macros?
- Does this project save cairo-profiler / cairo-debugger code mappings?
- And many more!

Well-known community projects like [`OpenZeppelin`](https://github.com/openZeppelin/cairo-contracts/), 
[`alexandria`](https://github.com/keep-starknet-strange/alexandria/) 
or [`starknet-staking`](https://github.com/starkware-libs/starknet-staking) can be a good starting point. 

### Profiling Scarb

While benchmarking is useful to detect performance regressions, it does not help you understand what causes them.
Scarb has built-in tracing support that can be used for quick profiling of a predefined set of events.
To enable tracing, set the env variable `SCARB_TRACING_PROFILE=1` before running Scarb.
Scarb will then output a file called `scarb-profile-{datetime}.json` in the current directory.

You can load this file in [perfetto](https://ui.perfetto.dev/) to navigate the events visually. 

### Further reading 

- [The Rust Performance Book](https://nnethercote.github.io/perf-book/)