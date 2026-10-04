use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

const MODEL: &str = r#"model "Services" {
  node api "API"; node worker "Worker"; api -> worker id=dispatch;
  view overview "Overview" { include api }
  view detail "Details" { include api worker; step entry { show api }; step request { show worker; show-edge dispatch } }
}"#;

struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "layup-scene-{}-{}",
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

#[test]
fn compile_defaults_to_stdout_and_matches_the_rust_scene_export() {
    let source = "diagram \"国际\" slide=wide { node api \"API\"; node worker \"Worker\"; api -> worker id=dispatch; step intro { show api }; step request { show worker; show-edge dispatch } }";
    let compiled = layup::compile(source).unwrap();
    let expected = layup::scene::export(&compiled).unwrap();
    let result = run(&["compile", "-"], source);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        String::from_utf8(result.stdout).unwrap().trim_end(),
        expected
    );
    let workspace = Workspace::new();
    let input = workspace.file("diagram.layup", source);
    let result = run(&["compile", input.to_str().unwrap()], "");
    assert!(result.status.success());
    assert_eq!(
        String::from_utf8(result.stdout).unwrap().trim_end(),
        expected
    );
    assert!(!input.with_extension("json").exists());
    let output = workspace.0.join("scene.json");
    assert!(
        run(
            &[
                "compile",
                input.to_str().unwrap(),
                "-o",
                output.to_str().unwrap()
            ],
            ""
        )
        .status
        .success()
    );
    assert_eq!(std::fs::read_to_string(output).unwrap(), expected);
}

#[test]
fn strict_compile_preserves_existing_output_and_errors_do_not_emit_json() {
    let warning = "diagram \"Small text\" slide=wide min-font-size=1000 { node api }";
    let workspace = Workspace::new();
    let output = workspace.file("scene.json", "previous scene");
    let result = run(
        &["compile", "-", "--strict", "-o", output.to_str().unwrap()],
        warning,
    );
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    assert_eq!(std::fs::read_to_string(&output).unwrap(), "previous scene");
    assert!(run(&["compile", "-"], warning).status.success());
    let result = run(&["compile", "-"], "diagram \"T\" { node 中文 aling=left }");
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("did you mean `align`?")
    );
}

#[test]
fn global_view_selection_is_honored_by_compile_render_check_lint_and_build() {
    for args in [
        ["--view", "detail", "compile", "-"],
        ["compile", "-", "--view", "detail"],
    ] {
        let result = run(&args, MODEL);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let text = String::from_utf8(result.stdout).unwrap();
        assert!(text.contains("\"selectedView\":\"detail\""));
        assert!(text.contains("\"id\":\"dispatch\""));
        assert!(text.contains("\"visibleEdges\":[\"dispatch\"]"));
    }
    let result = run(&["render", "-", "-o", "-", "--view", "overview"], MODEL);
    assert!(result.status.success());
    let svg = String::from_utf8(result.stdout).unwrap();
    assert!(svg.contains("data-id=\"api\""));
    assert!(!svg.contains("data-id=\"worker\""));
    assert!(
        run(&["check", "-", "--view", "detail", "--strict"], MODEL)
            .status
            .success()
    );
    assert!(
        run(&["lint", "-", "--view", "detail", "--strict"], MODEL)
            .status
            .success()
    );
    for args in [
        vec!["compile", "-", "--view", "missing"],
        vec!["check", "-", "--view", "missing"],
        vec!["lint", "-", "--view", "missing"],
    ] {
        let result = run(&args, MODEL);
        assert!(!result.status.success());
        assert!(
            String::from_utf8(result.stderr)
                .unwrap()
                .contains("unknown view")
        );
    }
    let workspace = Workspace::new();
    let input = workspace.file("services.layup", MODEL);
    assert!(
        run(
            &[
                "build",
                workspace.0.to_str().unwrap(),
                "--view",
                "detail",
                "--html",
                "--strict"
            ],
            ""
        )
        .status
        .success()
    );
    assert!(
        std::fs::read_to_string(input.with_extension("svg"))
            .unwrap()
            .contains("data-id=\"worker\"")
    );
    assert!(
        std::fs::read_to_string(input.with_extension("html"))
            .unwrap()
            .contains("createPresentation(svg")
    );
}
