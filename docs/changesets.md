# Changesets

Run `knope document-change` (Knope 0.22.4), or add a Markdown file here:

```markdown
---
baxe: patch
baxe-derive: patch
---

Describe the user-visible improvement in one sentence.

Add details below when needed.
```

Use **unquoted** crate names and `patch`, `minor`, or `major`. Knope 0.22.4
does not recognize quoted package names in these files. The first description
line becomes a heading in the changelog, so keep it short and complete.

Changes to `baxe-derive` must cover both crates because `baxe` re-exports its
macro. Test-only, workflow, and tooling changes do not require a release entry.
CI checks published source and dependency changes; version-only release PRs
are exempt. Follow Cargo's pre-1.0 compatibility conventions when choosing bumps.

Only these files drive version bumps; commit messages are ignored. Knope consumes
the files into per-crate changelogs when it prepares a release.

See [the release workflow](releases.md) for setup and operation.

Keep documentation outside `.changeset/`: Knope treats every Markdown file there
as a change file, including `README.md`.
