#!/usr/bin/env python3
"""Compare warm dependency, non-incremental cargo check times for macro consumers.

Usage: python3 scripts/benchmark_compile.py --baseline /path/to/baseline/checkout
Dependencies must already be cached locally. Only temporary fixtures are modified.
"""

import argparse
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import tempfile
import time


def fixture(count):
    variants = "\n".join(
        '#[baxe(status = StatusCode::BAD_REQUEST, tag = "bad_request", '
        'code = 400, message = "Invalid value {0}: {1:?}")]\n'
        f"Error{i}(String, Vec<usize>),"
        for i in range(count)
    )
    return (
        "#![allow(unused_variables, dead_code)]\n"
        "use baxe::{baxe_error, error, BackendError};\n"
        "use axum::{http::StatusCode, response::IntoResponse, Json};\n"
        "baxe_error!(String,);\n"
        f"#[error]\nenum Errors {{ {variants} }}\n"
    )


def check(directory, environment):
    start = time.perf_counter()
    result = subprocess.run(
        ["cargo", "check", "--offline", "--quiet"],
        cwd=directory, env=environment, capture_output=True, text=True,
    )
    if result.returncode:
        raise RuntimeError(result.stderr)
    return (time.perf_counter() - start) * 1000


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--samples", type=int, default=9)
    args = parser.parse_args()
    if args.samples < 1:
        parser.error("--samples must be positive")
    current = Path(__file__).resolve().parents[1]
    print(subprocess.check_output(["rustc", "--version"], text=True).strip())
    with tempfile.TemporaryDirectory(prefix="baxe-compile-") as temporary:
        root = Path(temporary)
        cases = []
        for label, repository in [("before", args.baseline.resolve()), ("after", current)]:
            directory = root / label
            (directory / "src").mkdir(parents=True)
            dependency = json.dumps(str(repository / "crates/core"))
            (directory / "Cargo.toml").write_text(
                '[package]\nname = "baxe-expansion-probe"\nversion = "0.0.0"\nedition = "2021"\n'
                '[dependencies]\n'
                f'baxe = {{ path = {dependency} }}\n'
                'axum = ">=0.7.0"\nserde = { version = "1", features = ["derive"] }\n'
            )
            shutil.copyfile(current / "Cargo.lock", directory / "Cargo.lock")
            environment = dict(
                os.environ, CARGO_INCREMENTAL="0", CARGO_TARGET_DIR=str(directory / "target"),
                RUSTC_WRAPPER="", RUSTC_WORKSPACE_WRAPPER="",
            )
            cases.append((label, directory, environment))

        for count in [10, 100, 500]:
            source = fixture(count)
            samples = {label: [] for label, _, _ in cases}
            for _, directory, environment in cases:
                (directory / "src/lib.rs").write_text(source)
                check(directory, environment)  # Build dependencies and warm the fixture.
            for sample in range(args.samples):
                # Alternate order to reduce systematic drift between before and after.
                for label, directory, environment in cases[::1 if sample % 2 == 0 else -1]:
                    (directory / "src/lib.rs").write_text(source + f"\n// sample {sample}\n")
                    samples[label].append(check(directory, environment))
            medians = {label: statistics.median(values) for label, values in samples.items()}
            reduction = (1 - medians["after"] / medians["before"]) * 100
            print(f"variants={count} before_ms={medians['before']:.1f} "
                  f"after_ms={medians['after']:.1f} reduction={reduction:.1f}%", flush=True)


if __name__ == "__main__":
    main()
