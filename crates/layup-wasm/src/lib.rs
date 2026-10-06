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

use base64::{Engine as _, engine::general_purpose::STANDARD};
use layup::Theme;

/// Options are `key=value` lines: `theme` (`light`, `dark`, `auto`),
/// `format` (`svg`, `html`, `embed`), `darkSelector` (SVG only), and repeated
/// `font` entries holding base64-encoded fallback OpenType/TrueType bytes.
/// `operation=lint` returns a diagnostics array; `operation=format` returns
/// formatted source in `output`. `operation=compile` returns versioned scene
/// JSON in `scene`. `operation=inspect` returns a typed partial document and
/// diagnostics without layout. `view` selects a reusable model view. All operations use
/// the same length-prefixed JSON ABI.
/// `input=graph` accepts versioned semantic graph JSON for render/compile.
pub fn render(src: &str, options: &str) -> String {
    let mut theme = Theme::Light;
    let mut format = "svg";
    let mut operation = "render";
    let mut graph_input = false;
    let mut dark_selector = None;
    let mut fonts = layup::text::Fonts::new();
    let mut compile_options = layup::CompileOptions::default();
    for (key, value) in options.lines().filter_map(|l| l.split_once('=')) {
        match key {
            "theme" => match Theme::parse(value) {
                Some(t) => theme = t,
                None => return error(None, &format!("unknown theme {value:?}")),
            },
            "format" if ["svg", "html", "embed"].contains(&value) => format = value,
            "operation" if ["render", "format", "lint", "compile", "inspect"].contains(&value) => {
                operation = value
            }
            "darkSelector" => dark_selector = Some(value),
            "view" => compile_options.view = Some(value.into()),
            "diagram" => compile_options.diagram = Some(value.into()),
            "input" if value == "graph" => graph_input = true,
            "font" => {
                let bytes = match STANDARD.decode(value) {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        return error(
                            None,
                            "font must be base64-encoded OpenType or TrueType bytes",
                        );
                    }
                };
                if let Err(e) = fonts.add_fallback(bytes) {
                    return error(None, &e.msg);
                }
            }
            _ => return error(None, &format!("unknown option {key}={value}")),
        }
    }
    if graph_input && !matches!(operation, "render" | "compile") {
        return error(
            None,
            "graph input supports only render and compile operations",
        );
    }
    if operation == "inspect" {
        return layup::document::inspect(src).to_string();
    }
    if operation == "format" {
        return match layup::format::format(src) {
            Ok(output) => format!("{{\"output\":{},\"warnings\":[]}}", string(&output)),
            Err(e) => source_error(&e),
        };
    }
    if operation == "lint" {
        let diagnostics = layup::lint::lint_with_options(src, &compile_options, &fonts);
        return format!(
            "{{\"diagnostics\":[{}]}}",
            diagnostics
                .iter()
                .map(layup::diagnostic::Diagnostic::json)
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    let result = if graph_input {
        layup::input::compile_json(src, &compile_options, &fonts)
    } else {
        layup::compile_with_options(src, &compile_options, &fonts)
    };
    let compiled = match result {
        Ok(c) => c,
        Err(e) => return source_error(&e),
    };
    if operation == "compile" {
        return match layup::scene::export(&compiled) {
            Ok(scene) => format!("{{\"scene\":{scene}}}"),
            Err(error) => source_error(&error),
        };
    }
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

fn source_error(error: &layup::Error) -> String {
    format!(
        "{{\"error\":{}}}",
        layup::diagnostic::Diagnostic::from_error(error).json()
    )
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
    fn compile_returns_versioned_scene_and_view_errors_with_source_spans() {
        let src = r#"model "Services" {
          node api "API"; node worker "Worker"; api -> worker id=request;
          view overview { include api }
          view detail slide=wide { include api worker; step entry { show api }; step request { show worker; show-edge request } }
        }"#;
        let compiled = layup::compile_with_options(
            src,
            &layup::CompileOptions {
                view: Some("detail".into()),
                ..Default::default()
            },
            &layup::text::Fonts::new(),
        )
        .unwrap();
        let expected = layup::scene::export(&compiled).unwrap();
        assert_eq!(
            render(src, "operation=compile\nview=detail"),
            format!("{{\"scene\":{expected}}}")
        );
        let error = render(src, "operation=compile\nview=missing");
        assert!(error.contains("\"code\":\"view/unknown\""));
        assert!(error.contains("\"span\":{\"start\":"));
        assert!(!error.contains("\"scene\":"));
        let lint = render(src, "operation=lint\nview=detail");
        assert_eq!(lint, "{\"diagnostics\":[]}");
    }

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
