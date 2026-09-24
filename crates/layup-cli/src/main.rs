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
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Clone, Copy, ValueEnum)]
enum ThemeArg {
    Light,
    Dark,
    Auto,
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
    /// Render a diagram to SVG (default) or interactive HTML.
    Render {
        /// Input `.layup` file, or `-` for stdin.
        input: PathBuf,
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
    /// Parse and lay out a diagram, reporting overflow, crossings and other problems.
    Check {
        /// Input `.layup` files, or Markdown files (`.md`) whose `layup` code blocks are checked.
        inputs: Vec<PathBuf>,
        /// Treat warnings as errors.
        #[arg(long)]
        strict: bool,
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
    match cli.cmd {
        Cmd::Render {
            input,
            output,
            html,
            theme,
            embed,
            strict,
        } => {
            let src = read(&input)?;
            let out =
                output.unwrap_or_else(|| input.with_extension(if html { "html" } else { "svg" }));
            let ok = render_one(&input, &src, &out, html, theme.into(), embed, strict)?;
            Ok(ok)
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
                        ok &= check_one(&input, Some(line), &block, strict);
                    }
                } else {
                    ok &= check_one(&input, None, &src, strict);
                }
            }
            Ok(ok)
        }
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
) -> Result<bool, Box<dyn std::error::Error>> {
    let compiled = match layup::compile(src) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}: error: {e}", input.display());
            return Ok(false);
        }
    };
    report(input, &compiled.warnings);
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
            compiled.scene.width,
            compiled.scene.height
        );
    }
    Ok(true)
}

/// Checks one diagram. `fence` is the Markdown line of the opening fence for
/// a diagram from a code block; its diagnostics then name Markdown lines.
fn check_one(input: &Path, fence: Option<usize>, src: &str, strict: bool) -> bool {
    let name = input.display();
    let at = |line: Option<usize>, msg: &str| match fence {
        Some(f) => format!("{name}:{}: {msg}", f + line.unwrap_or(0)),
        None => match line {
            Some(l) => format!("{name}: line {l}: {msg}"),
            None => format!("{name}: {msg}"),
        },
    };
    match layup::compile(src) {
        Ok(c) => {
            for w in &c.warnings {
                eprintln!("{}", at(w.line, &format!("warning: {}", w.msg)));
            }
            let label = fence.map_or(name.to_string(), |f| format!("{name}:{f}"));
            println!(
                "{label}: {} warnings, {}x{}",
                c.warnings.len(),
                c.scene.width,
                c.scene.height
            );
            !(strict && !c.warnings.is_empty())
        }
        Err(e) => {
            eprintln!("{}", at(e.line, &format!("error: {}", e.msg)));
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
    for (i, line) in src.lines().enumerate() {
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
                    body.push_str(line);
                    body.push('\n');
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
    out
}

fn report(input: &Path, warnings: &[layup::Warning]) {
    for w in warnings {
        eprintln!("{}: warning: {w}", input.display());
    }
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
