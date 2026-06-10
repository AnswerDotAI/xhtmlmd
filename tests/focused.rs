use xhtmlmd::{to_xhtml, Options};

#[test]
fn focused_dialect_fixture() {
    let input = include_str!("fixtures/dialect.md");
    let expected = include_str!("fixtures/dialect.xhtml");
    assert_eq!(
        normalize(&to_xhtml(input, &Options::default())),
        normalize(expected)
    );
}

#[test]
fn default_math_mode_is_brackets() {
    let html = to_xhtml("\\(y\\)\n\n\\[\nx^2\n\\]\n\n$x$", &Options::default());
    assert!(html.contains("<span class=\"math inline\">y</span>"));
    assert!(html.contains("<div class=\"math display\">x^2</div>"));
    assert!(html.contains("<p>$x$</p>"));
}

#[test]
fn math_modes_are_explicit() {
    let mut opt = Options::default();
    opt.math = xhtmlmd::MathMode::Off;
    let html = to_xhtml("$x$ and \\(y\\)", &opt);
    assert!(!html.contains("class=\"math"));
    assert!(html.contains("$x$ and (y)"));
    opt.math = xhtmlmd::MathMode::On;
    let html = to_xhtml(r"$x$ and \(y\) and \[z\]", &opt);
    assert!(!html.contains("class=\"math"));
    assert_eq!(html, "<p>$x$ and \\(y\\) and \\[z\\]</p>\n");
    assert_eq!(to_xhtml("\\[\nx^2\n\\]\n", &opt), "<p>\\[\nx^2\n\\]</p>\n");
    opt.math = xhtmlmd::MathMode::Brackets;
    let html = to_xhtml("$x$ and \\(y\\)", &opt);
    assert!(html.contains("$x$"));
    assert!(html.contains("<span class=\"math inline\">y</span>"));
    opt.math = xhtmlmd::MathMode::Dollars;
    assert!(to_xhtml("$x$", &opt).contains("<span class=\"math inline\">x</span>"));
}

#[test]
fn tagfilter_is_opt_in() {
    let input = "No <textarea>.\n\n<script>alert(1)</script>";
    let default = to_xhtml(input, &Options::default());
    assert!(default.contains("<textarea>"));
    assert!(default.contains("<script>"));

    let mut opt = Options::default();
    opt.tagfilter = true;
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
