# Development

## Prerequisites

- Rust toolchain
- Python 3.10+
- maturin

## Building

```bash
tools/build.sh
```

This builds the Rust CLI and copies `xhtmlmd` to `python/xhtmlmd.data/scripts/` so maturin includes it in wheels. Pass `release` for an optimized binary:

```bash
tools/build.sh release
```

For local Python development:

```bash
maturin develop
```

## Testing

```bash
tools/test.sh
```

## Release

Publishing is handled by GitHub Actions in `.github/workflows/ci.yml` and is triggered by pushing a tag matching `v*`.

Release flow is: release first, then bump.

1. Confirm tests pass:

```bash
tools/test.sh
```

2. Confirm the release version matches in both:

- `pyproject.toml` (`[project].version`)
- `Cargo.toml` (`[package].version`)

3. Tag that commit and push the tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

4. After pushing the release tag, bump both files to the next dev version and commit/push to `main` without a tag.

No local wheel build is required for release. CI builds wheels for Linux and macOS, creates a GitHub Release, and publishes to PyPI.
