#!/usr/bin/env python3
"""Preview the release plan; --publish uploads missing versions in dependency order."""
import argparse
import json
from pathlib import Path
import re
import subprocess
import tomllib
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
DIRECTORIES = ["crates/derive", "crates/core"]


def plan(root=ROOT):
    if any((root / ".changeset").glob("*.md")):
        raise ValueError("Prepare and merge the release PR before publishing pending changesets")
    packages = []
    for directory in DIRECTORIES:
        manifest = tomllib.loads((root / directory / "Cargo.toml").read_text())
        name, version = manifest["package"]["name"], manifest["package"]["version"]
        changelog = (root / directory / "CHANGELOG.md").read_text()
        if not re.search(r"^## " + re.escape(version) + r"(?:\s|$)", changelog, re.MULTILINE):
            raise ValueError(f"Missing release notes for {name} {version}")
        packages.append((name, version))
    return packages


def published(name, version):
    request = urllib.request.Request(
        f"https://crates.io/api/v1/crates/{name}/{version}",
        headers={"User-Agent": "baxe-release (https://github.com/zyphelabs/baxe)"},
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            metadata = json.load(response)["version"]
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return False
        raise  # Authentication, rate limit, and server errors are not missing versions.
    if metadata["num"] != version or metadata["yanked"]:
        raise ValueError(f"Unexpected or yanked registry version for {name} {version}")
    return True


def publish(packages):
    for name, version in packages:
        if published(name, version):
            print(f"Already published: {name} {version}", flush=True)
            continue
        subprocess.run(["cargo", "publish", "-p", name, "--locked"], cwd=ROOT, check=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--publish", action="store_true")
    args = parser.parse_args()
    packages = plan()
    if args.publish:
        publish(packages)
    else:
        for name, version in packages:
            print(f"Would publish {name} {version}")
