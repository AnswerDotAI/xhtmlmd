# Development

## Prerequisites

- Rust toolchain
- Python 3.10+
- maturin
- fastship>=0.0.11, installed by `pip install -e '.[dev]'`

## Building

For local development, build and install the extension into your environment:

```bash
maturin develop
```

`ship-rs-build` builds the distributable wheel. The `xhtmlmd` command is a Python console script (`python/xhtmlmd/__main__.py`) over the `to_xhtml` API; there is no separate Rust binary.

## Testing

```bash
pytest -q
```

All tests are Python (`tests/`), run against the built extension; there are no `cargo test` unit tests.

## Docs

```bash
python tools/gen_docs.py
python tools/gen_docs.py --check
```

## Release

Publishing is handled by GitHub Actions in `.github/workflows/ci.yml` and is triggered by pushing a tag matching `v*`.

Release flow is: release first, then bump.

1. Confirm tests pass:

```bash
pytest -q
```

2. Confirm the release version in `Cargo.toml` (`[package].version`). `pyproject.toml` gets the Python package version from Cargo via `dynamic = ["version"]`.

3. Tag that commit and push the tag:

```bash
ship-rs-release
```

4. After pushing the release tag, run `ship-rs-bump`, commit the `Cargo.toml` version bump, and push to `main` without a tag.

No local wheel build is required for release. CI builds wheels for Linux and macOS, creates a GitHub Release, and publishes to PyPI.
