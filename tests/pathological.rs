use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};
use xhtmlmd::{to_xhtml, Options};

const WATCHDOG: Duration = Duration::from_secs(15);

#[test]
fn nested_brackets_do_not_explode() {
    let n = 50_000;
    let input = format!("{}a{}", "[".repeat(n), "]".repeat(n));
    let (html, elapsed) = render_with_timeout(input);
    assert!(html.contains('a'));
    assert!(elapsed.as_secs() < 5);
}

#[test]
fn deep_blockquotes_are_bounded() {
    let n = 50_000;
    let input = format!("{}a", "> ".repeat(n));
    let (html, elapsed) = render_with_timeout(input);
    assert!(html.contains('a'));
    assert!(elapsed.as_secs() < 5);
}

#[test]
fn repeated_image_openers_are_linear_smoke() {
    let input = "![[]()".repeat(20_000);
    let (_, elapsed) = render_with_timeout(input);
    assert!(elapsed.as_secs() < 5);
}

#[test]
fn raw_html_balancing_is_linear_smoke() {
    let n = 20_000;
    let input = format!("{}{}", "<div>\n".repeat(n), "</div>\n".repeat(n));
    let (html, elapsed) = render_with_timeout(input);
    assert!(html.starts_with("<div>"));
    assert!(elapsed.as_secs() < 5);
}

#[test]
fn many_abbreviations_are_bounded() {
    let n = 2_000;
    let mut input = String::new();
    for i in 0..n {
        input.push_str(&format!("*[ABBR{i}]: title {i}\n"));
    }
    input.push('\n');
    for i in 0..n {
        input.push_str(&format!("ABBR{i} "));
    }
    let (html, elapsed) = render_with_timeout(input);
    assert!(html.contains("<abbr title=\"title 1999\">ABBR1999</abbr>"));
    assert!(elapsed.as_secs() < 5);
}

#[test]
fn nested_footnote_references_are_bounded() {
    let n = 1_000;
    let mut input = String::from("Start[^0]\n\n");
    for i in 0..n {
        input.push_str(&format!("[^{i}]: note"));
        if i + 1 < n {
            input.push_str(&format!("[^{}]", i + 1));
        }
        input.push_str("\n\n");
    }
    let (html, elapsed) = render_with_timeout(input);
    assert!(html.contains("fn-999"));
    assert!(elapsed.as_secs() < 5);
}

#[test]
fn markdown_html_close_scanning_skips_code_spans() {
    let n = 10_000;
    let input = format!(
        "<div markdown=\"1\">\n{}\n\nstill here\n</div>",
        "code `</div>` ".repeat(n)
    );
    let (html, elapsed) = render_with_timeout(input);
    assert!(html.contains("<p>still here</p>"));
    assert!(elapsed.as_secs() < 5);
}

fn render_with_timeout(input: String) -> (String, Duration) {
    let len = input.len();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let start = Instant::now();
        let html = to_xhtml(&input, &Options::default());
        let _ = tx.send((html, start.elapsed()));
    });
    match rx.recv_timeout(WATCHDOG) {
        Ok(result) => result,
        Err(RecvTimeoutError::Timeout) => panic!("parser exceeded {WATCHDOG:?} on {len} bytes"),
        Err(RecvTimeoutError::Disconnected) => {
            panic!("parser thread stopped before sending a result")
        }
    }
}
