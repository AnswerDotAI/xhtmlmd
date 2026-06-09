use std::time::Instant;
use xhtml_md_parser::{to_xhtml, Options};

#[test]
fn nested_brackets_do_not_explode() {
    let n = 50_000;
    let input = format!("{}a{}", "[".repeat(n), "]".repeat(n));
    let start = Instant::now();
    let html = to_xhtml(&input, &Options::default());
    assert!(html.contains('a'));
    assert!(start.elapsed().as_secs() < 5);
}

#[test]
fn deep_blockquotes_are_bounded() {
    let n = 50_000;
    let input = format!("{}a", "> ".repeat(n));
    let start = Instant::now();
    let html = to_xhtml(&input, &Options::default());
    assert!(html.contains('a'));
    assert!(start.elapsed().as_secs() < 5);
}

#[test]
fn repeated_image_openers_are_linear_smoke() {
    let input = "![[]()".repeat(160_000);
    let start = Instant::now();
    let _ = to_xhtml(&input, &Options::default());
    assert!(start.elapsed().as_secs() < 5);
}
