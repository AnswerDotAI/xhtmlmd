use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};
use xhtml_md_parser::{to_xhtml, Options};

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
