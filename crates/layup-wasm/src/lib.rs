//! A C ABI over `layup` for WebAssembly hosts, used by `packages/layup`.
//!
//! The host copies UTF-8 source and options into buffers from
//! [`layup_alloc`], calls [`layup_render`], and reads back a buffer holding
//! a little-endian `u32` length followed by that many bytes of JSON:
//! `{"output": "...", "warnings": [{"line": 3, "message": "..."}]}` or
//! `{"error": {"line": 3, "message": "..."}}`. The host frees every buffer
//! with [`layup_free`], passing the length it allocated (4 plus the JSON
//! length for results).

use std::fmt::Write as _;

use layup::Theme;

/// Options are `key=value` lines: `theme` (`light`, `dark`, `auto`),
/// `format` (`svg`, `html`, `embed`) and `darkSelector` (SVG only).
pub fn render(src: &str, options: &str) -> String {
    let mut theme = Theme::Light;
    let mut format = "svg";
    let mut dark_selector = None;
    for (key, value) in options.lines().filter_map(|l| l.split_once('=')) {
        match key {
            "theme" => match Theme::parse(value) {
                Some(t) => theme = t,
                None => return error(None, &format!("unknown theme {value:?}")),
            },
            "format" if ["svg", "html", "embed"].contains(&value) => format = value,
            "darkSelector" => dark_selector = Some(value),
            _ => return error(None, &format!("unknown option {key}={value}")),
        }
    }
    let compiled = match layup::compile(src) {
        Ok(c) => c,
        Err(e) => return error(e.line, &e.msg),
    };
    let output = match format {
        "svg" => layup::svg::render_with(
            &compiled,
            &layup::svg::Options {
                theme,
                dark_selector,
            },
        ),
        _ => layup::html::render(&compiled, theme, format == "embed"),
    };
    let mut json = format!("{{\"output\":{},\"warnings\":[", string(&output));
    for (i, w) in compiled.warnings.iter().enumerate() {
        let comma = if i > 0 { "," } else { "" };
        let _ = write!(json, "{comma}{}", diagnostic(w.line, &w.msg));
    }
    json.push_str("]}");
    json
}

fn error(line: Option<usize>, msg: &str) -> String {
    format!("{{\"error\":{}}}", diagnostic(line, msg))
}

fn diagnostic(line: Option<usize>, msg: &str) -> String {
    let line = line.map_or("null".to_string(), |l| l.to_string());
    format!("{{\"line\":{line},\"message\":{}}}", string(msg))
}

fn string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[unsafe(no_mangle)]
pub extern "C" fn layup_alloc(len: usize) -> *mut u8 {
    Box::into_raw(vec![0u8; len].into_boxed_slice()).cast()
}

/// # Safety
/// `ptr` and `len` must describe a buffer from `layup_alloc` or
/// `layup_render` that has not been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn layup_free(ptr: *mut u8, len: usize) {
    // SAFETY: the caller passes a boxed slice this module leaked, with its length.
    drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)) });
}

/// # Safety
/// Both pointer and length pairs must describe live buffers from
/// `layup_alloc` holding UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn layup_render(
    src: *const u8,
    src_len: usize,
    options: *const u8,
    options_len: usize,
) -> *mut u8 {
    // SAFETY: the caller guarantees both buffers are live allocations of these lengths.
    let (src, options) = unsafe {
        (
            std::slice::from_raw_parts(src, src_len),
            std::slice::from_raw_parts(options, options_len),
        )
    };
    let json = match (std::str::from_utf8(src), std::str::from_utf8(options)) {
        (Ok(src), Ok(options)) => render(src, options),
        _ => error(None, "input is not UTF-8"),
    };
    let mut out = (json.len() as u32).to_le_bytes().to_vec();
    out.extend(json.as_bytes());
    Box::into_raw(out.into_boxed_slice()).cast()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_json_with_warnings_and_errors() {
        let ok = render(r#"diagram "Hi" { node a "A" }"#, "theme=dark\nformat=svg");
        assert!(ok.starts_with(r#"{"output":"<svg"#) && ok.ends_with(r#""warnings":[]}"#));
        assert!(ok.contains(r#"class=\"layup dark\""#));
        assert!(render("diagram {", "").starts_with(r#"{"error":{"line":1,"#));
        assert!(render("", "theme=sepia").contains("unknown theme"));
        let class = render(r#"diagram "Hi" {}"#, "theme=auto\ndarkSelector=.dark");
        assert!(class.contains(":is(.dark) .layup.auto{") && !class.contains("@media"));
        assert_eq!(string("a\"\\\n\u{1}"), r#""a\"\\\n\u0001""#);
    }
}
