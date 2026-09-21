# Macro expansion performance

Baseline: `a1e4cf1dce505667f2f6df2bb272356d7977528e`.
Measured locally with `rustc 1.95.0 (59807616e 2026-04-14)`.

Changes:

- Emit one `write!` instead of `format!` followed by `write!` for variants with fields.
  This also removes the intermediate message allocation at runtime.
- Construct field bindings once and populate preallocated output vectors directly.
- Parse macro options from tokens and compare attribute identifiers without allocating strings.
- Return compiler diagnostics for malformed options/variant attributes instead of panicking.
  Both `logMessageWith=logger::error` and `logMessageWith = logger::error` are tested.

## Consumer compilation

Each fixture contains one enum whose variants have `String` and `Vec<usize>` fields.
The table reports median wall time over nine `cargo check` runs, including Cargo startup,
procedural macro execution, generated macro expansion, and type checking. Dependencies
are already built, incremental compilation and compiler cache wrappers are disabled, and fixture source changes
before every run to force a rebuild. Before/after order alternates between samples.
These are synthetic consumer checks, not clean builds or application-wide speedups.

| Variants | Before | After | Time reduction |
| --- | ---: | ---: | ---: |
| 10 | 97.6 ms | 96.0 ms | 1.6% |
| 100 | 148.4 ms | 133.1 ms | 10.3% |
| 500 | 406.4 ms | 344.1 ms | 15.3% |

The difference at ten variants is too small to establish a meaningful improvement.

Reproduce against a separate baseline checkout, with dependencies cached locally:

```sh
python3 scripts/benchmark_compile.py --baseline /path/to/baseline/checkout
```

## Expansion microbenchmark

```sh
cargo test -p baxe-derive --release --locked benchmark_expansion -- --ignored --nocapture
```

This exercises parsing and code generation through `proc_macro2`'s fallback backend.
It includes input token cloning and output destruction, but excludes the compiler
bridge and expansion/type checking of the generated code. Each size uses twenty
warmup iterations followed by nine batches of `10000 / variant_count` iterations.

For the baseline measurement only, the original macro body was wrapped in the same
private `expand` function accepting `proc_macro2::TokenStream`, replacing
`parse_macro_input!` with `syn::parse2`, and the same benchmark fixture was copied in.
No baseline generation or attribute parsing logic was changed. The consumer benchmark
used the unmodified baseline source.

Two passes were run in opposite orders. The first showed substantial host timing
variation (baseline 576/5494/27557 microseconds versus changed 519/3237/16848).
The second, reverse-order pass gave the following more conservative results; these
should still be treated as local observations rather than a guaranteed speedup.

| Variants | Before | After | Time reduction | Output tokens before / after |
| --- | ---: | ---: | ---: | ---: |
| 10 | 381.7 µs | 356.1 µs | 6.7% | 995 / 905 |
| 100 | 3415.6 µs | 3187.4 µs | 6.7% | 8015 / 7115 |
| 500 | 16975.7 µs | 15973.4 µs | 5.9% | 39215 / 34715 |

Token counts recursively count token trees, including groups, before generated macros
are expanded by rustc. The fixture emits nine fewer token trees per variant.

## Validation

`cargo test --workspace --lib --tests --locked` passes, covering existing behavior,
all three variant shapes, metadata expressions using field bindings, response status,
logging, hidden messages, formatting specifiers, and malformed input diagnostics.

The baseline `cargo test --workspace --locked` failed the documentation example
in `crates/core/src/types.rs`: `baxe_error!` was not imported. The release automation
integration fixes and separates the examples, so the full suite now passes.
