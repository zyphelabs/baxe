#!/usr/bin/env python3
"""Require explicit changesets for published code changes relative to a PR base."""
import re
import subprocess
import sys
import tomllib

PACKAGES = {"baxe": "crates/core", "baxe-derive": "crates/derive"}


def git(*args):
    return subprocess.check_output(["git", *args], text=True)


def manifest_without_release_versions(text):
    data = tomllib.loads(text)
    data.get("package", {}).pop("version", None)
    for name in PACKAGES:
        dependency = data.get("dependencies", {}).get(name)
        if isinstance(dependency, dict) and "path" in dependency:
            dependency.pop("version", None)
    return data


def entries(text):
    parts = text.split("---", 2)
    if len(parts) != 3 or parts[0].strip() or not parts[2].strip():
        raise ValueError("Changesets need YAML front matter and a description")
    result = set()
    for line in parts[1].splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        match = re.fullmatch(r"([\w-]+): (patch|minor|major)", line)
        if not match or match[1] not in PACKAGES:
            raise ValueError(f"Expected a known crate and patch/minor/major, got: {line}")
        result.add(match[1])
    if not result:
        raise ValueError("Changeset must name at least one crate")
    return result


def check(base, head):
    ancestor = git("merge-base", base, head).strip()
    changed = git("diff", "--name-only", ancestor, head).splitlines()
    existing = set(git("ls-tree", "-r", "--name-only", head).splitlines())
    covered = set()
    for path in existing:
        if path.startswith(".changeset/") and path.endswith(".md") and path in existing:
            declared = entries(git("show", f"{head}:{path}"))
            if path in changed:
                covered.update(declared)
    required = set()
    for package, directory in PACKAGES.items():
        if any(path.startswith(directory + "/src/") for path in changed):
            required.add(package)
        manifest = directory + "/Cargo.toml"
        if manifest in changed:
            old = manifest_without_release_versions(git("show", f"{ancestor}:{manifest}"))
            new = manifest_without_release_versions(git("show", f"{head}:{manifest}"))
            if old != new:
                required.add(package)
    if "baxe-derive" in required:
        required.add("baxe")  # The public crate re-exports the procedural macro.
    missing = required - covered
    if missing:
        raise ValueError("Add a changeset for: " + ", ".join(sorted(missing)))
    print("Changeset coverage OK")


if __name__ == "__main__":
    check(*sys.argv[1:])
