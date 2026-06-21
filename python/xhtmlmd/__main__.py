"Command-line interface: render Markdown (file or stdin) to an XHTML fragment."
import sys

from . import to_xhtml

USAGE = ("usage: xhtmlmd [--math=off|on|brackets|dollars] [file.md]\n\n"
    "Reads Markdown from a file or stdin and writes XHTML fragment output. Math defaults to brackets.")

def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    math, file = "brackets", None
    for arg in argv:
        if arg in ("--math=off", "--math=on", "--math=brackets", "--math=dollars"): math = arg.split("=", 1)[1]
        elif arg in ("-h", "--help"): print(USAGE); return
        elif arg.startswith("--"): print(f"unknown option: {arg}", file=sys.stderr); sys.exit(2)
        else: file = arg
    src = open(file, encoding="utf-8").read() if file else sys.stdin.read()
    sys.stdout.write(to_xhtml(src, math=math))

if __name__ == "__main__": main()
