use xhtml_md_parser::{to_xhtml, Options};

#[test]
fn focused_extensions_fixture() {
    let input = include_str!("fixtures/extensions.md");
    let expected = include_str!("fixtures/extensions.xhtml");
    assert_eq!(normalize(&to_xhtml(input, &Options::default())), normalize(expected));
}

#[test]
fn math_modes_are_explicit() {
    let mut opt = Options::default();
    opt.math = xhtml_md_parser::MathMode::Off;
    assert!(to_xhtml("$x$ and \\(y\\)", &opt).contains("$x$ and \\(y\\)"));
    opt.math = xhtml_md_parser::MathMode::Brackets;
    let html = to_xhtml("$x$ and \\(y\\)", &opt);
    assert!(html.contains("$x$"));
    assert!(html.contains("<span class=\"math inline\">y</span>"));
    opt.math = xhtml_md_parser::MathMode::Dollars;
    assert!(to_xhtml("$x$", &opt).contains("<span class=\"math inline\">x</span>"));
}

fn normalize(s: &str) -> String { s.lines().map(str::trim_end).collect::<Vec<_>>().join("\n").trim().to_string() }
