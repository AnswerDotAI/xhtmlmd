use std::env;
use std::fs;
use std::io::{self, Read};
use xhtmlmd::{to_xhtml, MathMode, Options};

fn main() {
    let mut options = Options::default();
    let mut file = None;
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--math=off" => options.math = MathMode::Off,
            "--math=on" => options.math = MathMode::On,
            "--math=brackets" => options.math = MathMode::Brackets,
            "--math=dollars" => options.math = MathMode::Dollars,
            "--help" | "-h" => {
                help();
                return;
            }
            _ if arg.starts_with("--") => {
                eprintln!("unknown option: {arg}");
                std::process::exit(2);
            }
            _ => file = Some(arg),
        }
    }
    let mut input = String::new();
    if let Some(path) = file {
        input =
            fs::read_to_string(path).unwrap_or_else(|e| die(&format!("failed to read input: {e}")));
    } else {
        io::stdin()
            .read_to_string(&mut input)
            .unwrap_or_else(|e| die(&format!("failed to read stdin: {e}")));
    }
    print!("{}", to_xhtml(&input, &options));
}

fn help() {
    println!("usage: xhtmlmd [--math=off|on|brackets|dollars] [file.md]\n\nReads Markdown from a file or stdin and writes XHTML fragment output. Math defaults to brackets.");
}
fn die(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1)
}
