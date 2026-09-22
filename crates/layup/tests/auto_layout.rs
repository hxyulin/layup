use layup::{Compiled, compile};

fn rect(c: &Compiled, id: &str) -> layup::layout::Rect {
    c.scene.nodes[c.scene.node(id).unwrap()].rect
}

#[test]
fn diamond_layers_follow_edges_not_declaration_order() {
    let c = compile(
        r#"diagram "Flow" layout=auto {
        node sink "Sink"
        node left "Left"
        node source "Source"
        node right "Right"
        source -> left
        source -> right
        left -> sink
        right -> sink
    }"#,
    )
    .unwrap();
    assert!(rect(&c, "source").bottom() < rect(&c, "left").y);
    assert_eq!(rect(&c, "left").y, rect(&c, "right").y);
    assert!(rect(&c, "left").x < rect(&c, "right").x);
    assert!(rect(&c, "left").bottom() < rect(&c, "sink").y);
}

#[test]
fn manual_flow_remains_the_default() {
    let src = r#"diagram "Flow" { node b "B"; node a "A"; a -> b }"#;
    let c = compile(src).unwrap();
    assert!(rect(&c, "b").bottom() < rect(&c, "a").y);
    let manual = compile(&src.replacen("{", "layout=manual {", 1)).unwrap();
    assert_eq!(rect(&c, "a"), rect(&manual, "a"));
}

#[test]
fn cycles_and_disconnected_nodes_are_deterministic() {
    let src = r#"diagram "Cycle" layout=auto {
        node a "A"; node b "B"; node isolated "Isolated"; node sink "Sink"
        a -> b; b -> a; b -> sink
    }"#;
    let a = compile(src).unwrap();
    let b = compile(src).unwrap();
    assert_eq!(rect(&a, "a").y, rect(&a, "b").y);
    assert!(rect(&a, "isolated").y > rect(&a, "sink").bottom());
    assert!(rect(&a, "sink").y > rect(&a, "a").bottom());
    for id in ["a", "b", "isolated", "sink"] {
        assert_eq!(rect(&a, id), rect(&b, id));
    }
}

#[test]
fn explicit_rows_and_sections_are_boundaries() {
    let c = compile(
        r#"diagram "Authored" layout=auto {
        row 2:1 { node b "B"; node a "A" }
        section "Fixed" { node d "D"; node c "C"; c -> d }
        a -> b
        c -> a
    }"#,
    )
    .unwrap();
    assert_eq!(rect(&c, "a").y, rect(&c, "b").y);
    assert!(rect(&c, "b").w > rect(&c, "a").w);
    assert!(rect(&c, "c").y > rect(&c, "a").bottom());
    assert!(rect(&c, "d").y > rect(&c, "c").bottom());
}

#[test]
fn nested_edges_order_their_containers() {
    let c = compile(
        r#"diagram "Nested" layout=auto {
        node downstream "Downstream" { node sink "Sink" }
        node upstream "Upstream" { node b "B"; node a "A"; a -> b }
        b -> sink
    }"#,
    )
    .unwrap();
    assert!(rect(&c, "upstream").bottom() < rect(&c, "downstream").y);
    assert!(rect(&c, "a").bottom() < rect(&c, "b").y);
}

#[test]
fn left_arrows_reverse_flow_and_undirected_edges_do_not_rank() {
    let c = compile(
        r#"diagram "Directions" layout=auto {
        node a "A"; node b "B"; node c "C"; node d "D"
        a <- b; b -- c; c <-> d
    }"#,
    )
    .unwrap();
    assert!(rect(&c, "b").bottom() < rect(&c, "a").y);
    assert_eq!(rect(&c, "b").y, rect(&c, "c").y);
    assert_eq!(rect(&c, "c").y, rect(&c, "d").y);
}

#[test]
fn wide_layers_wrap_and_invalid_modes_fail() {
    let c = compile(
        r#"diagram "Wide" layout=auto {
        node a "A"; node b "B"; node c "C"; node d "D"
    }"#,
    )
    .unwrap();
    assert_eq!(rect(&c, "a").y, rect(&c, "c").y);
    assert!(rect(&c, "d").y > rect(&c, "c").bottom());
    assert!(compile(r#"diagram "Bad" layout=oops {}"#).is_err());
}
