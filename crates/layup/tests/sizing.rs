mod support;
use layup::{compile, layout::Rect};

fn rect(c: &layup::Compiled, id: &str) -> Rect {
    c.scene.nodes[support::node(&c.scene, id).unwrap()].rect
}

#[test]
fn automatic_width_fits_long_horizontal_machines_in_both_directions() {
    for direction in ["right", "left"] {
        let mut source = format!(
            "diagram main \"Long workflow\" type=state-machine flow-direction={direction} {{ initial start; final end;"
        );
        for i in 0..14 {
            source.push_str(&format!("state q{i} \"Processing step {i}\";"));
        }
        source.push_str("start -> q0;");
        for i in 0..13 {
            source.push_str(&format!("q{i} -> q{} \"next\";", i + 1));
        }
        source.push_str("q13 -> q0 \"restart\"; q13 -> end \"finish\" }");
        let c = compile(&source).unwrap();
        assert!(c.warnings.is_empty(), "{:?}", c.warnings);
        assert!(c.scene.width > 2500.0);
        assert_eq!(c.scene.width, c.diagram.width);
        let parsed = layup::model::build(&source).unwrap();
        let laid_out = layup::layout::layout(&parsed, &mut Vec::new());
        assert_eq!(laid_out.width, c.scene.width);
        for i in 0..14 {
            assert!(rect(&c, &format!("q{i}")).w >= 150.0);
        }
        assert!(
            c.scene
                .edges
                .iter()
                .flat_map(|e| &e.points)
                .all(|&(x, y)| x >= 0.0 && x <= c.scene.width && y >= 0.0 && y <= c.scene.height)
        );
    }
}

#[test]
fn wide_trees_grow_and_compact_nodes_use_content_widths() {
    for direction in ["down", "up", "right", "left"] {
        let mut source = format!(
            "diagram main \"Outcomes\" type=graph layout=auto flow-direction={direction} {{ node root \"Which outcome?\" style=decision;"
        );
        for i in 0..16 {
            source.push_str(&format!(
                "node q{i} \"Outcome {i}\" style=terminal; root -> q{i} \"{i}\";"
            ));
        }
        source.push('}');
        let c = compile(&source).unwrap();
        assert!(c.warnings.is_empty(), "{:?}", c.warnings);
        let outcomes: Vec<_> = (0..16).map(|i| rect(&c, &format!("q{i}"))).collect();
        for (i, a) in outcomes.iter().enumerate() {
            for b in &outcomes[i + 1..] {
                assert!(
                    a.right() <= b.x || b.right() <= a.x || a.bottom() <= b.y || b.bottom() <= a.y
                );
            }
        }
        if direction == "down" {
            assert!(c.scene.width > 2200.0);
        }
    }
    let c = compile(
        r#"diagram main "Sizes" type=state-machine flow-direction=right {
  initial s
  state a "A"
  state b "Processing application"
  final end
  transition connection-1 ::s -> ::a
  transition connection-2 ::a -> ::b
  transition connection-3 ::b -> ::end
}"#,
    )
    .unwrap();
    assert!(rect(&c, "a").w < rect(&c, "b").w);
    assert_eq!(rect(&c, "s").w, 20.0);
}

#[test]
fn explicit_width_and_manual_layout_keep_authored_canvas() {
    for head in ["width=1100 layout=auto", "width=1100"] {
        let c = compile(&format!(
            "diagram main \"Authored\" type=graph {head} {{\n  node a \"A\" style=process\n  node b \"B\" style=process\n  ::a -> ::b\n}}"
        ))
        .unwrap();
        assert_eq!(c.scene.width, 1100.0);
        assert_eq!(rect(&c, "a").w, 240.0);
    }
}

#[test]
fn automatic_sizes_use_supplied_font_metrics() {
    let mut source = String::from(
        "diagram main \"Metrics\" type=state-machine flow-direction=right { initial start; final end;",
    );
    for i in 0..10 {
        source.push_str(&format!("state q{i} \"中中中中中中中中\";"));
    }
    source.push_str("start -> q0;");
    for i in 0..9 {
        source.push_str(&format!("q{i} -> q{};", i + 1));
    }
    source.push_str("q9 -> end }");
    let default = compile(&source).unwrap();
    let mut fonts = layup::text::Fonts::new();
    fonts
        .add_fallback(include_bytes!("fonts/Fallback.ttf").as_slice())
        .unwrap();
    let custom = layup::compile_with_fonts(&source, &fonts).unwrap();
    assert!(default.warnings.is_empty(), "{:?}", default.warnings);
    assert!(custom.warnings.is_empty(), "{:?}", custom.warnings);
    assert!(custom.scene.width < default.scene.width);
    assert!(rect(&custom, "q0").w < rect(&default, "q0").w);
}
