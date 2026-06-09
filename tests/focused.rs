use xhtml_md_parser::{to_xhtml, Options};

#[test]
fn focused_extensions_fixture() {
    let input = include_str!("fixtures/extensions.md");
    let expected = include_str!("fixtures/extensions.xhtml");
    assert_eq!(
        normalize(&to_xhtml(input, &Options::default())),
        normalize(expected)
    );
}

#[test]
fn math_modes_are_explicit() {
    let mut opt = Options::default();
    opt.math = xhtml_md_parser::MathMode::Off;
    let html = to_xhtml("$x$ and \\(y\\)", &opt);
    assert!(!html.contains("class=\"math"));
    assert!(html.contains("$x$ and (y)"));
    opt.math = xhtml_md_parser::MathMode::Brackets;
    let html = to_xhtml("$x$ and \\(y\\)", &opt);
    assert!(html.contains("$x$"));
    assert!(html.contains("<span class=\"math inline\">y</span>"));
    opt.math = xhtml_md_parser::MathMode::Dollars;
    assert!(to_xhtml("$x$", &opt).contains("<span class=\"math inline\">x</span>"));
}

#[test]
fn tagfilter_is_opt_in() {
    let input = "No <textarea>.\n\n<script>alert(1)</script>";
    let default = to_xhtml(input, &Options::default());
    assert!(default.contains("<textarea>"));
    assert!(default.contains("<script>"));

    let mut opt = Options::default();
    opt.extensions.tagfilter = true;
    let filtered = to_xhtml(input, &opt);
    assert!(filtered.contains("&lt;textarea>"));
    assert!(filtered.contains("&lt;script>"));
    assert!(filtered.contains("&lt;/script>"));
}

fn normalize(s: &str) -> String {
    s.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}
