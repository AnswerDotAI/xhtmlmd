"Command-line interface: render Markdown (file or stdin) to an XHTML fragment."
import sys

from . import to_xhtml

USAGE = ("usage: xhtmlmd [--math=off|on|brackets|dollars] [--balance] [--no-table-widths] [file.md]\n\n"
    "Reads Markdown from a file or stdin and writes XHTML fragment output. Math defaults to brackets.\n"
    "--balance closes unclosed raw HTML tags and drops stray closing tags.\n"
    "--no-table-widths disables <colgroup> column widths from pipe-table separator dash counts.")

def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    math, balance, table_widths, file = "brackets", False, True, None
    for arg in argv:
        if arg in ("--math=off", "--math=on", "--math=brackets", "--math=dollars"): math = arg.split("=", 1)[1]
        elif arg == "--balance": balance = True
        elif arg == "--no-table-widths": table_widths = False
        elif arg in ("-h", "--help"): print(USAGE); return
        elif arg.startswith("--"): print(f"unknown option: {arg}", file=sys.stderr); sys.exit(2)
        else: file = arg
    src = open(file, encoding="utf-8").read() if file else sys.stdin.read()
    sys.stdout.write(to_xhtml(src, math=math, balance=balance, table_widths=table_widths))

if __name__ == "__main__": main()
