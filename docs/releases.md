# Releases with Knope

Baxe uses Knope 0.22.4, following the backend's changeset-driven release model.
`knope.toml` defines `baxe` and `baxe-derive` separately, including their lockfile
entries and the dependency from `baxe` to `baxe-derive`.

## One-time GitHub setup

Install the existing Zyphe release bot GitHub App on `zyphelabs/baxe`, with
Contents and Pull requests read/write permissions. Make these settings available
to this repository (repository settings or organization settings with repository access):

- Variable `ZYPHE_RELEASE_BOT_APP_ID`.
- Secret `ZYPHE_RELEASE_BOT_APP_PRIVATE_KEY`.
- Secret `CRATES_TOKEN`, authorized to publish both crates on crates.io.

Use the App token for release PRs so their CI checks are triggered. No credentials
are needed for local release preparation or the ordinary PR checks. The
`ci` job matches the required `ci` status check on `main`.

## Normal flow

1. Add a file under `.changeset/` with `knope document-change` alongside code changes.
2. PR CI runs Rust tests including doctests, Clippy, release-tool tests, Knope config
   validation, and a check that published code changes have changeset coverage.
3. After a push to `main` passes tests, CI prepares pending changesets with Knope.
   Versions, internal dependency requirements, and `Cargo.lock` are updated together;
   per-crate changelogs are generated and consumed changesets removed.
4. CI opens or updates the `release` branch PR. Review its versions and release notes.
   Additional merges to `main` update the same release PR.
5. Merging that same-repository PR into `main` starts **Publish** on the exact merge
   commit. It reruns tests, publishes `baxe-derive` before `baxe`, then uses Knope to
   create GitHub releases and tags such as `baxe-derive/v0.1.7` and `baxe/v0.1.7`.

The macro optimization changeset prepares version 0.1.7 for both crates. Adding
Knope does not itself change their current versions or publish a release.

## Local verification

Install the pinned CLI with `cargo install knope --version 0.22.4 --locked`.

```sh
knope --dry-run prepare-release
python3 -B -m unittest discover -s scripts -p 'test_*.py'
python3 scripts/check_changesets.py origin/main HEAD
```

To inspect the actual generated release diff, run `knope prepare-release` in a
disposable checkout. Unlike `--dry-run`, this reads and consumes the changesets,
updates files, and stages them. It does not commit, tag, upload, or create a PR.
Then run `cargo metadata --locked --no-deps --format-version 1` and
`python3 scripts/publish.py` to validate the lockfile and preview publication order.

## Retries and manual operation

Rerun a failed Publish workflow to retry its original merge commit. The publisher
skips versions already present on crates.io, so a failure after the first crate
uploads can resume without re-uploading it. Registry errors abort rather than being
treated as missing versions. GitHub releases are created only after publication.

The **ci** workflow can be dispatched on `main` to prepare a release without a new
push. **Publish** can be dispatched on `main` after a release is prepared; it refuses
pending changesets or missing version-specific changelog entries. Prefer rerunning
the original workflow if `main` has advanced since the release merge.

Tag pushes no longer start publication, avoiding duplicate publishes when Knope
creates the two per-crate tags. No secrets or publishing steps run on unmerged PRs.
