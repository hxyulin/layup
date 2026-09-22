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
        /// Input `.layup` files.
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
                match layup::compile(&src) {
                    Ok(c) => {
                        report(&input, &c.warnings);
                        if strict && !c.warnings.is_empty() {
                            ok = false;
                        }
                        println!(
                            "{}: {} warnings, {}x{}",
                            input.display(),
                            c.warnings.len(),
                            c.scene.width,
                            c.scene.height
                        );
                    }
                    Err(e) => {
                        eprintln!("{}: error: {e}", input.display());
                        ok = false;
                    }
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
