"Command-line interface: render Markdown (file or stdin) to an XHTML fragment."
import sys

from . import to_xhtml, sample_md

USAGE = ("usage: xhtmlmd [--math=off|on|brackets|dollars] [--balance] [--sample] [file.md]\n\n"
    "Reads Markdown from a file or stdin and writes XHTML fragment output. Math defaults to brackets.\n"
    "--balance closes unclosed raw HTML tags and drops stray closing tags.\n"
    "--sample renders the packaged feature-sample document as a demo.")

def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    math, balance, sample, file = "brackets", False, False, None
    for arg in argv:
        if arg in ("--math=off", "--math=on", "--math=brackets", "--math=dollars"): math = arg.split("=", 1)[1]
        elif arg == "--balance": balance = True
        elif arg == "--sample": sample = True
        elif arg in ("-h", "--help"): print(USAGE); return
        elif arg.startswith("--"): print(f"unknown option: {arg}", file=sys.stderr); sys.exit(2)
        else: file = arg
    src = sample_md() if sample else open(file, encoding="utf-8").read() if file else sys.stdin.read()
    sys.stdout.write(to_xhtml(src, math=math, balance=balance))

if __name__ == "__main__": main()
