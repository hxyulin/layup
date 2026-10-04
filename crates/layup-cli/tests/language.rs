use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "layup-language-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self, name: &str, source: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, source).unwrap();
        path
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn run(args: &[&str], source: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_layup"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(source.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}
fn path(path: &Path) -> &str {
    path.to_str().unwrap()
}

#[test]
fn formatter_supports_stdin_check_and_preflighted_batch_writes() {
    let source = "diagram \"T\"{node a;node b;a->b}// tail\n";
    let formatted = layup::format::format(source).unwrap();
    let result = run(&["fmt"], source);
    assert!(result.status.success());
    assert_eq!(String::from_utf8(result.stdout).unwrap(), formatted);
    assert!(run(&["fmt", "--check"], &formatted).status.success());
    assert!(!run(&["fmt", "--check"], source).status.success());
    assert!(!run(&["fmt", "--write"], source).status.success());

    let workspace = Workspace::new();
    let good = workspace.file("good.layup", source);
    let bad_source = "diagram \"T\" { node a href= }";
    let bad = workspace.file("bad.layup", bad_source);
    assert!(
        !run(&["fmt", "--write", path(&good), path(&bad)], "")
            .status
            .success()
    );
    assert_eq!(std::fs::read_to_string(&good).unwrap(), source);
    assert_eq!(std::fs::read_to_string(&bad).unwrap(), bad_source);
    std::fs::write(&bad, source).unwrap();
    assert!(
        run(&["fmt", "--write", path(&good), path(&bad)], "")
            .status
            .success()
    );
    assert_eq!(std::fs::read_to_string(&good).unwrap(), formatted);
    assert!(
        run(&["fmt", "--check", path(&good), path(&bad)], "")
            .status
            .success()
    );
    assert!(!run(&["fmt", path(&good), path(&bad)], "").status.success());
}

#[test]
fn lint_reports_multiple_errors_and_strict_warnings_with_exit_status() {
    let invalid = "diagram \"T\" {\n node a href=\n a ->\n node b \"bad\\q\"\n}";
    let result = run(&["lint", "-", "--json"], invalid);
    assert!(!result.status.success());
    let json = String::from_utf8(result.stdout).unwrap();
    assert_eq!(json.matches("\"severity\":\"error\"").count(), 3);
    assert!(json.contains("parse/attribute-value") && json.contains("lex/escape"));
    assert!(result.stderr.is_empty());
    let warning = "diagram \"T\" { style unused blue; node a }";
    assert!(run(&["lint", "-"], warning).status.success());
    assert!(!run(&["lint", "-", "--strict"], warning).status.success());
    assert!(run(&["check", "-", "--strict"], warning).status.success());
    let result = run(
        &["render", "-", "-o", "-"],
        "diagram \"T\" {\n node 中文 aling=left\n}",
    );
    let shown = String::from_utf8(result.stderr).unwrap();
    assert!(
        shown.contains("-:2:10:")
            && shown.contains("^^^^^")
            && shown.contains("did you mean `align`?"),
        "{shown}"
    );
}

#[test]
fn markdown_json_spans_are_absolute_including_crlf_and_multibyte_prefixes() {
    let workspace = Workspace::new();
    let source = "# 标题\r\n\r\n```layup\r\ndiagram \"T\" {\r\n node a href=\r\n}\r\n```\r\n";
    let file = workspace.file("guide.md", source);
    let result = run(&["lint", path(&file), "--json"], "");
    assert!(!result.status.success());
    let json = String::from_utf8(result.stdout).unwrap();
    assert!(json.contains("\"fenceLine\":3"));
    assert!(json.contains("\"line\":5"));
    let start = source.find("href=").unwrap() + 5;
    assert!(
        json.contains(&format!("\"start\":{start},\"end\":{}", start + 2)),
        "{json}"
    );
    let shown = run(&["lint", path(&file)], "");
    assert!(
        String::from_utf8(shown.stderr)
            .unwrap()
            .contains(&format!("{}:5:14:", file.display()))
    );
    // CommonMark accepts an unclosed fence through the end of the document.
    let file = workspace.file("unclosed.md", "```layup\ndiagram \"T\" {\n node a href=\n}");
    assert!(!run(&["lint", path(&file)], "").status.success());
}
