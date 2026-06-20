# Development

## Prerequisites

- Rust toolchain
- Python 3.10+
- maturin
- fastship

## Building

```bash
ship-rs-build
```

This builds the Rust CLI and copies `xhtmlmd` to `python/xhtmlmd.data/scripts/` so maturin includes it in wheels.

For local Python development:

```bash
maturin develop
```

## Testing

```bash
ship-rs-test
```

## Release

Publishing is handled by GitHub Actions in `.github/workflows/ci.yml` and is triggered by pushing a tag matching `v*`.

Release flow is: release first, then bump.

1. Confirm tests pass:

```bash
ship-rs-test
```

2. Confirm the release version in `Cargo.toml` (`[package].version`). `pyproject.toml` gets the Python package version from Cargo via `dynamic = ["version"]`.

3. Tag that commit and push the tag:

```bash
ship-rs-release
```

4. After pushing the release tag, run `ship-rs-bump`, commit the `Cargo.toml` version bump, and push to `main` without a tag.

No local wheel build is required for release. CI builds wheels for Linux and macOS, creates a GitHub Release, and publishes to PyPI.
