use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::{MathMode, Options};

#[pyfunction]
#[pyo3(signature = (
    markdown,
    *,
    math = "brackets",
    tagfilter = false,
    max_inline_depth = None,
    max_block_depth = None,
    max_link_paren_depth = None
))]
fn to_xhtml(
    markdown: &str,
    math: &str,
    tagfilter: bool,
    max_inline_depth: Option<usize>,
    max_block_depth: Option<usize>,
    max_link_paren_depth: Option<usize>,
) -> PyResult<String> {
    let mut options = Options {
        math: parse_math_mode(math)?,
        tagfilter,
        ..Options::default()
    };
    if let Some(depth) = max_inline_depth {
        options.max_inline_depth = depth;
    }
    if let Some(depth) = max_block_depth {
        options.max_block_depth = depth;
    }
    if let Some(depth) = max_link_paren_depth {
        options.max_link_paren_depth = depth;
    }
    Ok(crate::to_xhtml(markdown, &options))
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(to_xhtml, m)?)?;
    Ok(())
}

fn parse_math_mode(mode: &str) -> PyResult<MathMode> {
    match mode {
        "off" => Ok(MathMode::Off),
        "on" => Ok(MathMode::On),
        "brackets" => Ok(MathMode::Brackets),
        "dollars" => Ok(MathMode::Dollars),
        _ => Err(PyValueError::new_err(
            "math must be 'off', 'on', 'brackets', or 'dollars'",
        )),
    }
}
