"Command-line interface: render Markdown (file or stdin) to an MDHTML fragment."
import sys

from . import to_mdhtml, sample_md

USAGE = ("usage: mdhtml [options] [file.md]\n\n"
    "Reads Markdown from a file or stdin and writes MDHTML fragment output. Math defaults to brackets.\n"
    "--auto-ids derives ids for headings; --implicit-figures promotes image-only paragraphs.\n"
    "--no-bare-autolinks leaves bare URLs and email addresses as text.\n"
    "--sample renders the packaged feature-sample document as a demo.")

def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    math, bare_autolinks, auto_ids, implicit_figures, sample, file = "brackets", True, False, False, False, None
    for arg in argv:
        if arg in ("--math=off", "--math=on", "--math=brackets", "--math=dollars"): math = arg.split("=", 1)[1]
        elif arg == "--no-bare-autolinks": bare_autolinks = False
        elif arg == "--auto-ids": auto_ids = True
        elif arg == "--implicit-figures": implicit_figures = True
        elif arg == "--sample": sample = True
        elif arg in ("-h", "--help"):
            print(USAGE)
            return
        elif arg.startswith("--"):
            print(f"unknown option: {arg}", file=sys.stderr)
            sys.exit(2)
        else: file = arg
    src = sample_md() if sample else open(file, encoding="utf-8").read() if file else sys.stdin.read()
    sys.stdout.write(to_mdhtml(src, math=math, bare_autolinks=bare_autolinks, auto_ids=auto_ids,
        implicit_figures=implicit_figures))

if __name__ == "__main__": main()
