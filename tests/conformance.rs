use ego_tree::NodeRef;
use scraper::{Html, Node};
use std::collections::BTreeMap;
use std::env;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;
use xhtml_md_parser::{to_xhtml, MathMode, Options};

const CMARK_GFM_SPEC: &str = include_str!("source/cmark-gfm/spec.txt");
const CMARK_GFM_EXTENSIONS: &str = include_str!("source/cmark-gfm/extensions.txt");
const MF_SPEC: &str = include_str!("source/mf.txt");
const FENCE: &str = "````````````````````````````````";
const MAX_FAILURES_TO_PRINT: usize = 20;
const CASE_TIMEOUT: Duration = Duration::from_secs(15);

#[test]
fn markdown_spec_report() {
    let mut cases = [
        parse_cmark_examples("spec.txt", CMARK_GFM_SPEC),
        parse_cmark_examples("extensions.txt", CMARK_GFM_EXTENSIONS),
        parse_cmark_examples("mf.txt", MF_SPEC),
    ]
    .concat();
    if let Ok(section) = env::var("XHTML_MD_CONFORMANCE_SECTION") {
        cases.retain(|case| case.section == section);
    }
    if let Ok(example) = env::var("XHTML_MD_CONFORMANCE_EXAMPLE") {
        if let Ok(example) = example.parse::<usize>() {
            cases.retain(|case| case.example == example);
        }
    }
    if let Ok(limit) = env::var("XHTML_MD_CONFORMANCE_LIMIT") {
        if let Ok(limit) = limit.parse::<usize>() {
            cases.truncate(limit);
        }
    }
    let trace = env::var_os("XHTML_MD_CONFORMANCE_TRACE").is_some();
    let mut by_section: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut failures = Vec::new();

    for case in &cases {
        if trace {
            eprintln!(
                "{} example {} ({})",
                case.source, case.example, case.section
            );
        }
        let actual = render_with_timeout(case);
        let expected_tree = normalize_html(&case.html);
        let actual_tree = normalize_html(&actual);
        let entry = by_section.entry(case.section.clone()).or_default();
        entry.0 += 1;
        if expected_tree == actual_tree {
            entry.1 += 1;
        } else {
            failures.push(Failure {
                source: case.source,
                example: case.example,
                section: case.section.clone(),
                markdown: case.markdown.clone(),
                expected: render_nodes(&expected_tree),
                actual: render_nodes(&actual_tree),
            });
        }
    }

    println!(
        "markdown specs: {} passed, {} failed, {} total",
        cases.len() - failures.len(),
        failures.len(),
        cases.len()
    );
    for (section, (total, passed)) in &by_section {
        println!("{passed:>3}/{total:<3} {section}");
    }
    for failure in failures.iter().take(MAX_FAILURES_TO_PRINT) {
        println!(
            "\n{} example {} ({})\nmarkdown:\n{}\nexpected:\n{}\nactual:\n{}",
            failure.source,
            failure.example,
            failure.section,
            failure.markdown.trim_end(),
            failure.expected,
            failure.actual
        );
    }

    assert!(
        failures.is_empty(),
        "{} of {} markdown spec examples failed; first {} failures printed",
        failures.len(),
        cases.len(),
        failures.len().min(MAX_FAILURES_TO_PRINT)
    );
}

fn render_with_timeout(case: &CmarkExample) -> String {
    let markdown = case.markdown.clone();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut options = Options::default();
        options.math = MathMode::Off;
        let html = to_xhtml(&markdown, &options);
        let _ = tx.send(html);
    });
    match rx.recv_timeout(CASE_TIMEOUT) {
        Ok(html) => html,
        Err(RecvTimeoutError::Timeout) => panic!(
            "{} example {} ({}) exceeded {:?}\nmarkdown:\n{}",
            case.source,
            case.example,
            case.section,
            CASE_TIMEOUT,
            case.markdown.trim_end()
        ),
        Err(RecvTimeoutError::Disconnected) => panic!(
            "{} example {} ({}) stopped before sending a result",
            case.source, case.example, case.section
        ),
    }
}

#[derive(Clone, Debug)]
struct CmarkExample {
    source: &'static str,
    example: usize,
    section: String,
    markdown: String,
    html: String,
}

#[derive(Clone, Debug)]
struct Failure {
    source: &'static str,
    example: usize,
    section: String,
    markdown: String,
    expected: String,
    actual: String,
}

fn parse_cmark_examples(source_name: &'static str, source: &str) -> Vec<CmarkExample> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum State {
        Text,
        Markdown,
        Html,
    }

    let mut state = State::Text;
    let mut section = String::new();
    let mut markdown = String::new();
    let mut html = String::new();
    let mut extensions = Vec::new();
    let mut example = 0usize;
    let mut out = Vec::new();

    for line in source.split_inclusive('\n') {
        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
        match state {
            State::Text => {
                if let Some(rest) = trimmed.strip_prefix(&format!("{FENCE} example")) {
                    state = State::Markdown;
                    extensions = rest.split_whitespace().map(str::to_string).collect();
                    markdown.clear();
                    html.clear();
                } else if let Some(heading) = heading_text(trimmed) {
                    section = heading;
                }
            }
            State::Markdown => {
                if trimmed == "." {
                    state = State::Html;
                } else {
                    markdown.push_str(line);
                }
            }
            State::Html => {
                if trimmed == FENCE {
                    example += 1;
                    if !extensions.iter().any(|ext| ext == "disabled") {
                        out.push(CmarkExample {
                            source: source_name,
                            example,
                            section: section.clone(),
                            markdown: markdown.replace('→', "\t"),
                            html: html.replace('→', "\t"),
                        });
                    }
                    state = State::Text;
                } else {
                    html.push_str(line);
                }
            }
        }
    }

    out
}

fn heading_text(line: &str) -> Option<String> {
    let hashes = line.bytes().take_while(|b| *b == b'#').count();
    if hashes > 0 && line.as_bytes().get(hashes) == Some(&b' ') {
        Some(line[hashes + 1..].trim().to_string())
    } else {
        None
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum NormNode {
    Element {
        tag: String,
        attrs: Vec<(String, String)>,
        children: Vec<NormNode>,
    },
    Text(String),
    Comment(String),
    Doctype(String),
    ProcessingInstruction {
        target: String,
        data: String,
    },
}

fn normalize_html(input: &str) -> Vec<NormNode> {
    let html = Html::parse_fragment(input);
    let mut children = html
        .root_element()
        .children()
        .filter_map(|node| normalize_node(node, false))
        .collect::<Vec<_>>();
    normalize_text_edges(&mut children, true, false);
    children
}

fn normalize_node(node: NodeRef<'_, Node>, in_pre: bool) -> Option<NormNode> {
    match node.value() {
        Node::Document | Node::Fragment => None,
        Node::Text(text) => {
            let text = if in_pre {
                text.to_string()
            } else {
                collapse_whitespace(text)
            };
            if text.is_empty() {
                None
            } else {
                Some(NormNode::Text(text))
            }
        }
        Node::Comment(comment) => Some(NormNode::Comment(comment.to_string())),
        Node::Doctype(doctype) => Some(NormNode::Doctype(doctype.name().to_string())),
        Node::ProcessingInstruction(pi) => Some(NormNode::ProcessingInstruction {
            target: pi.target.to_string(),
            data: pi.data.to_string(),
        }),
        Node::Element(element) => {
            let tag = element.name().to_string();
            let next_in_pre = in_pre || tag == "pre";
            let mut attrs = element
                .attrs()
                .map(|(name, value)| (name.to_string(), normalize_attr_value(name, value)))
                .collect::<Vec<_>>();
            attrs.sort();
            let mut children = node
                .children()
                .filter_map(|child| normalize_node(child, next_in_pre))
                .collect::<Vec<_>>();
            normalize_text_edges(&mut children, is_block_tag(&tag), next_in_pre);
            Some(NormNode::Element {
                tag,
                attrs,
                children,
            })
        }
    }
}

fn normalize_attr_value(name: &str, value: &str) -> String {
    if is_boolean_attr(name) && (value.is_empty() || value.eq_ignore_ascii_case(name)) {
        String::new()
    } else {
        value.to_string()
    }
}

fn normalize_text_edges(children: &mut Vec<NormNode>, parent_is_block: bool, in_pre: bool) {
    if in_pre {
        return;
    }
    let child_is_block = children
        .iter()
        .map(|child| matches!(child, NormNode::Element { tag, .. } if is_block_tag(tag)))
        .collect::<Vec<_>>();
    let last = children.len().saturating_sub(1);
    for i in 0..children.len() {
        let Some(NormNode::Text(text)) = children.get_mut(i) else {
            continue;
        };
        if (parent_is_block && i == 0) || i.checked_sub(1).is_some_and(|prev| child_is_block[prev])
        {
            *text = text.trim_start().to_string();
        }
        if (parent_is_block && i == last) || child_is_block.get(i + 1).copied().unwrap_or(false) {
            *text = text.trim_end().to_string();
        }
    }
    children.retain(|child| !matches!(child, NormNode::Text(text) if text.is_empty()));
}

fn collapse_whitespace(input: &str) -> String {
    let mut out = String::new();
    let mut in_space = false;
    for ch in input.chars() {
        if ch.is_whitespace() {
            if !in_space {
                out.push(' ');
                in_space = true;
            }
        } else {
            out.push(ch);
            in_space = false;
        }
    }
    out
}

fn render_nodes(nodes: &[NormNode]) -> String {
    nodes.iter().map(render_node).collect::<Vec<_>>().join("")
}

fn render_node(node: &NormNode) -> String {
    match node {
        NormNode::Text(text) => text.clone(),
        NormNode::Comment(comment) => format!("<!--{comment}-->"),
        NormNode::Doctype(name) => format!("<!DOCTYPE {name}>"),
        NormNode::ProcessingInstruction { target, data } => format!("<?{target} {data}>"),
        NormNode::Element {
            tag,
            attrs,
            children,
        } => {
            let attrs = attrs
                .iter()
                .map(|(name, value)| {
                    if value.is_empty() {
                        format!(" {name}")
                    } else {
                        format!(" {name}={value:?}")
                    }
                })
                .collect::<String>();
            format!("<{tag}{attrs}>{}</{tag}>", render_nodes(children))
        }
    }
}

fn is_boolean_attr(name: &str) -> bool {
    matches!(
        name,
        "allowfullscreen"
            | "async"
            | "autofocus"
            | "autoplay"
            | "checked"
            | "controls"
            | "default"
            | "defer"
            | "disabled"
            | "formnovalidate"
            | "hidden"
            | "ismap"
            | "loop"
            | "multiple"
            | "muted"
            | "novalidate"
            | "open"
            | "readonly"
            | "required"
            | "reversed"
            | "selected"
    )
}

fn is_block_tag(tag: &str) -> bool {
    matches!(
        tag,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "body"
            | "br"
            | "button"
            | "canvas"
            | "caption"
            | "col"
            | "colgroup"
            | "dd"
            | "details"
            | "div"
            | "dl"
            | "dt"
            | "embed"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hgroup"
            | "hr"
            | "iframe"
            | "li"
            | "map"
            | "object"
            | "ol"
            | "output"
            | "p"
            | "pre"
            | "progress"
            | "section"
            | "table"
            | "tbody"
            | "td"
            | "textarea"
            | "tfoot"
            | "th"
            | "thead"
            | "tr"
            | "ul"
            | "video"
    )
}
