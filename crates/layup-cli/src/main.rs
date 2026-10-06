use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "layup",
    version,
    about = "Authored-layout diagrams rendered to SVG and interactive HTML"
)]
struct Cli {
    /// Fallback OpenType/TrueType font to measure and embed. Repeat for multiple fonts.
    #[arg(long = "font", global = true)]
    fonts: Vec<PathBuf>,
    /// Select a named view from a reusable model document.
    #[arg(long, global = true)]
    view: Option<String>,
    /// Select a diagram from a revision-one document (defaults to the first).
    #[arg(long, global = true)]
    diagram: Option<String>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Clone, Copy, ValueEnum)]
enum ThemeArg {
    Light,
    Dark,
    Auto,
}

#[derive(Clone, Copy, ValueEnum)]
enum InputFormat {
    /// Infer graph JSON from a .json extension; otherwise read DSL.
    Auto,
    Dsl,
    Graph,
}

impl From<ThemeArg> for layup::Theme {
    fn from(t: ThemeArg) -> Self {
        match t {
            ThemeArg::Light => layup::Theme::Light,
            ThemeArg::Dark => layup::Theme::Dark,
            ThemeArg::Auto => layup::Theme::Auto,
        }
    }
}

#[derive(Subcommand)]
enum Cmd {
    /// Inspect a named document, preserving opaque bodies and recovering syntax errors.
    Inspect {
        /// Input DSL file, or `-` for stdin.
        input: PathBuf,
    },
    /// Render a diagram to SVG (default) or interactive HTML.
    Render {
        /// Input `.layup` or graph `.json` file, or `-` for stdin.
        input: PathBuf,
        /// Use graph for structured JSON on stdin; auto detects .json files.
        #[arg(long, default_value = "auto")]
        input_format: InputFormat,
        /// Output path; defaults to the input name with `.svg` or `.html`. `-` writes to stdout.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Emit an interactive HTML page (pan, zoom, hover highlighting, iframe messaging).
        #[arg(long)]
        html: bool,
        /// Color theme. `auto` follows the viewer's `prefers-color-scheme`.
        #[arg(long, default_value = "light")]
        theme: ThemeArg,
        /// Hide the toolbar in HTML output, for embedding in an iframe.
        #[arg(long)]
        embed: bool,
        /// Treat warnings as errors.
        #[arg(long)]
        strict: bool,
    },
    /// Compile a diagram to versioned scene JSON, writing stdout by default.
    Compile {
        /// Input `.layup` or graph `.json` file, or `-` for stdin.
        input: PathBuf,
        #[arg(long, default_value = "auto")]
        input_format: InputFormat,
        /// Output JSON path; defaults to stdout. `-` also writes stdout.
        #[arg(short, long, default_value = "-")]
        output: PathBuf,
        /// Treat warnings as errors; do not write output when strict checks fail.
        #[arg(long)]
        strict: bool,
    },
    /// Parse and lay out a diagram, reporting overflow, crossings and other problems.
    Check {
        /// Input `.layup`/graph `.json` files, or Markdown (`.md`) with `layup` code blocks.
        inputs: Vec<PathBuf>,
        /// Treat warnings as errors.
        #[arg(long)]
        strict: bool,
    },
    /// Check syntax, semantics, layout and authoring rules without writing output.
    Lint {
        /// DSL or Markdown files; `-` reads DSL from stdin.
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        /// Emit one JSON array of diagnostics, with file names and source spans.
        #[arg(long)]
        json: bool,
        /// Fail on warnings as well as errors.
        #[arg(long)]
        strict: bool,
    },
    /// Format DSL source, preserving comments and quoted text.
    Fmt {
        /// DSL files; defaults to stdin. Multiple files require --check or --write.
        #[arg(default_value = "-")]
        inputs: Vec<PathBuf>,
        /// Fail if any file differs from canonical formatting; do not write.
        #[arg(long, conflicts_with = "write")]
        check: bool,
        /// Replace files after all inputs pass syntax checks; cannot write stdin.
        #[arg(long, conflicts_with = "check")]
        write: bool,
    },
    /// Render every `.layup` file in a directory tree (`.svg` next to each source).
    Build {
        /// Directory to scan.
        dir: PathBuf,
        /// Also write an interactive `.html` beside each `.svg`.
        #[arg(long)]
        html: bool,
        #[arg(long, default_value = "light")]
        theme: ThemeArg,
        #[arg(long)]
        strict: bool,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<bool, Box<dyn std::error::Error>> {
    let mut fonts = layup::text::Fonts::new();
    for path in &cli.fonts {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        fonts
            .add_fallback(bytes)
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let options = layup::CompileOptions {
        view: cli.view,
        diagram: cli.diagram,
    };
    match cli.cmd {
        Cmd::Render {
            input,
            input_format,
            output,
            html,
            theme,
            embed,
            strict,
        } => {
            let src = read(&input)?;
            let out =
                output.unwrap_or_else(|| input.with_extension(if html { "html" } else { "svg" }));
            let ok = render_one(
                &input,
                &src,
                &out,
                html,
                theme.into(),
                embed,
                strict,
                &fonts,
                &options,
                input_format,
            )?;
            Ok(ok)
        }
        Cmd::Compile {
            input,
            input_format,
            output,
            strict,
        } => {
            let source = read(&input)?;
            let compiled = match compile_input(&input, &source, input_format, &options, &fonts) {
                Ok(compiled) => compiled,
                Err(error) => {
                    eprintln!(
                        "{}",
                        layup::diagnostic::Diagnostic::from_error(&error).display(
                            &source,
                            &input.display().to_string(),
                            0
                        )
                    );
                    return Ok(false);
                }
            };
            report(&input, &source, 0, &compiled.warnings);
            if strict && !compiled.warnings.is_empty() {
                return Ok(false);
            }
            let scene = layup::scene::export(&compiled)?;
            if output.as_os_str() == "-" {
                println!("{scene}");
            } else {
                std::fs::write(&output, scene).map_err(|e| format!("{}: {e}", output.display()))?;
                eprintln!("wrote {}", output.display());
            }
            Ok(true)
        }
        Cmd::Inspect { input } => {
            let result = layup::document::inspect(&read(&input)?);
            println!("{result:#}");
            Ok(!result["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d["severity"] == "error"))
        }
        Cmd::Check { inputs, strict } => {
            let mut ok = true;
            for input in inputs {
                let src = read(&input)?;
                if input
                    .extension()
                    .is_some_and(|e| e == "md" || e == "markdown")
                {
                    let blocks = fences(&src);
                    if blocks.is_empty() {
                        println!("{}: no layup blocks", input.display());
                    }
                    for (line, block) in blocks {
                        ok &= check_one(&input, Some(line), &block, strict, &fonts, &options);
                    }
                } else {
                    ok &= check_one(&input, None, &src, strict, &fonts, &options);
                }
            }
            Ok(ok)
        }
        Cmd::Lint {
            inputs,
            json,
            strict,
        } => lint_files(&inputs, json, strict, &fonts, &options),
        Cmd::Fmt {
            inputs,
            check,
            write,
        } => format_files(&inputs, check, write),
        Cmd::Build {
            dir,
            html,
            theme,
            strict,
        } => {
            let mut files = Vec::new();
            collect(&dir, &mut files)?;
            files.sort();
            let mut ok = true;
            for input in files {
                let src = read(&input)?;
                ok &= render_one(
                    &input,
                    &src,
                    &input.with_extension("svg"),
                    false,
                    theme.into(),
                    false,
                    strict,
                    &fonts,
                    &options,
                    InputFormat::Auto,
                )?;
                if html {
                    ok &= render_one(
                        &input,
                        &src,
                        &input.with_extension("html"),
                        true,
                        theme.into(),
                        false,
                        strict,
                        &fonts,
                        &options,
                        InputFormat::Auto,
                    )?;
                }
            }
            Ok(ok)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_one(
    input: &Path,
    src: &str,
    out: &Path,
    html: bool,
    theme: layup::Theme,
    embed: bool,
    strict: bool,
    fonts: &layup::text::Fonts,
    options: &layup::CompileOptions,
    input_format: InputFormat,
) -> Result<bool, Box<dyn std::error::Error>> {
    let compiled = match compile_input(input, src, input_format, options, fonts) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "{}",
                layup::diagnostic::Diagnostic::from_error(&e).display(
                    src,
                    &input.display().to_string(),
                    0
                )
            );
            return Ok(false);
        }
    };
    report(input, src, 0, &compiled.warnings);
    if strict && !compiled.warnings.is_empty() {
        return Ok(false);
    }
    let text = if html {
        layup::html::render(&compiled, theme, embed)
    } else {
        layup::svg::render(&compiled, theme)
    };
    if out.as_os_str() == "-" {
        print!("{text}");
    } else {
        std::fs::write(out, text).map_err(|e| format!("{}: {e}", out.display()))?;
        eprintln!(
            "wrote {} ({}x{})",
            out.display(),
            compiled.viewport().0,
            compiled.viewport().1
        );
    }
    Ok(true)
}

fn compile_input(
    input: &Path,
    source: &str,
    format: InputFormat,
    options: &layup::CompileOptions,
    fonts: &layup::text::Fonts,
) -> Result<layup::Compiled, layup::Error> {
    if matches!(format, InputFormat::Graph)
        || matches!(format, InputFormat::Auto) && input.extension().is_some_and(|e| e == "json")
    {
        layup::input::compile_json(source, options, fonts)
    } else {
        layup::compile_with_options(source, options, fonts)
    }
}

/// Checks one diagram. `fence` is the Markdown line of the opening fence for
/// a diagram from a code block; its diagnostics then name Markdown lines.
fn check_one(
    input: &Path,
    fence: Option<usize>,
    src: &str,
    strict: bool,
    fonts: &layup::text::Fonts,
    options: &layup::CompileOptions,
) -> bool {
    let name = input.display();
    match compile_input(input, src, InputFormat::Auto, options, fonts) {
        Ok(c) => {
            report(input, src, fence.unwrap_or(0), &c.warnings);
            let label = fence.map_or(name.to_string(), |f| format!("{name}:{f}"));
            println!(
                "{label}: {} warnings, {}x{}",
                c.warnings.len(),
                c.viewport().0,
                c.viewport().1
            );
            !(strict && !c.warnings.is_empty())
        }
        Err(e) => {
            eprintln!(
                "{}",
                layup::diagnostic::Diagnostic::from_error(&e).display(
                    src,
                    &name.to_string(),
                    fence.unwrap_or(0)
                )
            );
            false
        }
    }
}

/// `layup` code blocks in Markdown, with the 1-based line of each opening
/// fence. Fences follow CommonMark: three or more backticks or tildes, closed
/// by at least as many of the same character, so an example fence nested in a
/// longer one is not a diagram.
fn fences(src: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut open: Option<(char, usize, bool, usize, String)> = None;
    for (i, original) in src.split_inclusive('\n').enumerate() {
        let line = original.trim_end_matches(['\r', '\n']);
        let trimmed = line.trim_start();
        let marker = trimmed.chars().next().filter(|c| *c == '`' || *c == '~');
        let run = marker.map_or(0, |m| trimmed.chars().take_while(|c| *c == m).count());
        match &mut open {
            Some((c, len, layup, start, body)) => {
                if marker == Some(*c) && run >= *len && trimmed[run..].trim().is_empty() {
                    if *layup {
                        out.push((*start, std::mem::take(body)));
                    }
                    open = None;
                } else if *layup {
                    body.push_str(original);
                }
            }
            None if run >= 3 => {
                let info = trimmed[run..].trim();
                let is_layup = info.split_whitespace().next() == Some("layup");
                if marker == Some('~') || !info.contains('`') {
                    open = Some((marker.unwrap_or('`'), run, is_layup, i + 1, String::new()));
                }
            }
            None => {}
        }
    }
    if let Some((_, _, true, start, body)) = open {
        out.push((start, body));
    }
    out
}

fn report(input: &Path, source: &str, offset: usize, warnings: &[layup::Warning]) {
    let parsed = layup::document::parse_recovering(source);
    let mut spans = Vec::new();
    fn spans_in(
        body: &[layup::document::GraphStatement],
        spans: &mut Vec<layup::diagnostic::Span>,
    ) {
        for statement in body {
            spans.push(statement.span());
            spans_in(statement.children(), spans);
        }
    }
    for diagram in &parsed.document.diagrams {
        spans.push(diagram.span);
        if let Some(body) = diagram.body.statements() {
            spans_in(body, &mut spans);
        }
    }
    for w in warnings {
        let span = w
            .line
            .and_then(|line| spans.iter().find(|span| span.line == line))
            .copied();
        let diagnostic = layup::diagnostic::Diagnostic {
            severity: layup::diagnostic::Severity::Warning,
            code: "layout",
            message: w.msg.clone(),
            span,
            line: w.line,
            help: None,
            related: Vec::new(),
        };
        eprintln!(
            "{}",
            diagnostic.display(source, &input.display().to_string(), offset)
        );
    }
}

fn lint_files(
    inputs: &[PathBuf],
    json: bool,
    strict: bool,
    fonts: &layup::text::Fonts,
    options: &layup::CompileOptions,
) -> Result<bool, Box<dyn std::error::Error>> {
    let mut ok = true;
    let mut output = Vec::new();
    for input in inputs {
        let source = read(input)?;
        let blocks = if input
            .extension()
            .is_some_and(|e| e == "md" || e == "markdown")
        {
            fences(&source)
        } else {
            vec![(0, source.clone())]
        };
        for (offset, block) in blocks {
            let diagnostics = layup::lint::lint_with_options(&block, options, fonts);
            for mut d in diagnostics {
                ok &= d.severity != layup::diagnostic::Severity::Error && !strict;
                if json {
                    let byte_offset: usize = source
                        .split_inclusive('\n')
                        .take(offset)
                        .map(str::len)
                        .sum();
                    d.line = d.line.map(|l| l + offset);
                    if let Some(s) = &mut d.span {
                        s.start += byte_offset;
                        s.end += byte_offset;
                        s.line += offset;
                        s.end_line += offset;
                    }
                    for (s, _) in &mut d.related {
                        s.start += byte_offset;
                        s.end += byte_offset;
                        s.line += offset;
                        s.end_line += offset;
                    }
                    output.push(format!(
                        "{{\"file\":{},\"fenceLine\":{},\"diagnostic\":{}}}",
                        layup::diagnostic::quote(&input.display().to_string()),
                        if offset == 0 {
                            "null".into()
                        } else {
                            offset.to_string()
                        },
                        d.json()
                    ));
                } else {
                    eprintln!(
                        "{}",
                        d.display(&block, &input.display().to_string(), offset)
                    );
                }
            }
        }
    }
    if json {
        println!("[{}]", output.join(","));
    }
    Ok(ok)
}

fn format_files(
    inputs: &[PathBuf],
    check: bool,
    write: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    if inputs.len() > 1 && !check && !write {
        return Err("multiple formatter inputs require --check or --write".into());
    }
    if write && inputs.iter().any(|p| p.as_os_str() == "-") {
        return Err("--write requires file paths; use `layup fmt -` for stdin".into());
    }
    if inputs.iter().filter(|p| p.as_os_str() == "-").count() > 1 {
        return Err("stdin can be read only once".into());
    }
    let mut prepared = Vec::new();
    let mut valid = true;
    for input in inputs {
        let source = read(input)?;
        match layup::format::format(&source) {
            Ok(formatted) => prepared.push((input, source, formatted)),
            Err(e) => {
                eprintln!(
                    "{}",
                    layup::diagnostic::Diagnostic::from_error(&e).display(
                        &source,
                        &input.display().to_string(),
                        0
                    )
                );
                valid = false;
            }
        }
    }
    // No source is overwritten when any input has invalid syntax.
    if !valid {
        return Ok(false);
    }
    let mut clean = true;
    for (input, source, formatted) in prepared {
        if check {
            if source != formatted {
                eprintln!("{}: needs formatting", input.display());
                clean = false;
            }
        } else if write {
            if source != formatted {
                std::fs::write(input, formatted)
                    .map_err(|e| format!("{}: {e}", input.display()))?;
                eprintln!("formatted {}", input.display());
            }
        } else {
            print!("{formatted}");
        }
    }
    Ok(clean)
}

fn read(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    if path.as_os_str() == "-" {
        let mut s = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut s)?;
        return Ok(s);
    }
    Ok(std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?)
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let p = entry?.path();
        if p.is_dir() {
            if p.file_name()
                .is_some_and(|n| n == "target" || n.to_string_lossy().starts_with('.'))
            {
                continue;
            }
            collect(&p, out)?;
        } else if p.extension().is_some_and(|e| e == "layup") {
            out.push(p);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::fences;

    #[test]
    fn finds_layup_fences_but_not_nested_examples() {
        let md = "# T\n\n```layup\ndiagram \"A\" {}\n```\n\n````md\n```layup\nnot a diagram\n```\n````\n\n  ~~~~layup extra\n  diagram \"B\" {}\n  ~~~~\n```js\nx\n```\n";
        let found = fences(md);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0], (3, "diagram \"A\" {}\n".to_string()));
        assert_eq!(found[1].0, 13);
        assert!(found[1].1.contains("\"B\""));
    }
}
