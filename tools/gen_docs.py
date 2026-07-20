#!/usr/bin/env python3
"Generate checked-in documentation artifacts."
import argparse, sys
from pathlib import Path

from xhtmlmd import to_xhtml, sample_md

ROOT = Path(__file__).resolve().parents[1]
SAMPLE_HTML = ROOT / "docs" / "sample.html"


def main(argv=None):
    p = argparse.ArgumentParser(description="Generate docs/sample.html from the packaged sample.md.")
    p.add_argument("--check", action="store_true", help="fail if generated docs differ from checked-in files")
    args = p.parse_args(argv)
    html = to_xhtml(sample_md())
    if args.check:
        if SAMPLE_HTML.read_text(encoding="utf-8") == html: return 0
        print("docs/sample.html is out of date; run python tools/gen_docs.py", file=sys.stderr)
        return 1
    SAMPLE_HTML.write_text(html, encoding="utf-8")
    return 0


if __name__ == "__main__": sys.exit(main())
