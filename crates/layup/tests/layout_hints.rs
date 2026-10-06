mod support;
use layup::{Compiled, compile};

fn rect(c: &Compiled, id: &str) -> layup::layout::Rect {
    c.scene.nodes[support::node(&c.scene, id).unwrap()].rect
}

#[test]
fn below_places_a_peer_without_adding_an_edge() {
    let c = compile(
        r#"diagram main "Hints" type=graph layout=auto {
  node a "A" after=::b
  node b "B"
}"#,
    )
    .unwrap();
    assert!(rect(&c, "a").y > rect(&c, "b").bottom());
    assert!(c.scene.edges.is_empty());
}

#[test]
fn same_layer_overrides_inferred_rank() {
    let c = compile(
        r#"diagram main "Hints" type=graph layout=auto {
  node root "Root"
  node cache "Cache"
  node worker "Worker" same-rank=::cache
  ::root -> ::worker
}"#,
    )
    .unwrap();
    assert_eq!(rect(&c, "cache").y, rect(&c, "worker").y);
    assert!(rect(&c, "root").bottom() < rect(&c, "cache").y);
}

#[test]
fn beside_keeps_a_chain_adjacent_even_when_declared_out_of_order() {
    let c = compile(
        r#"diagram main "Hints" type=graph layout=auto {
  node c "C" beside=::b
  node other "Other"
  node a "A"
  node b "B" beside=::a
}"#,
    )
    .unwrap();
    assert_eq!(rect(&c, "a").y, rect(&c, "c").y);
    assert!(rect(&c, "a").right() < rect(&c, "b").x);
    assert!(rect(&c, "b").right() < rect(&c, "c").x);
    assert!(rect(&c, "other").y > rect(&c, "a").bottom());
}

#[test]
fn explicit_same_layer_groups_do_not_wrap_at_three() {
    let c = compile(
        r#"diagram main "Hints" type=graph layout=auto {
  node a "A"
  node b "B" same-rank=::a
  node c "C" same-rank=::a
  node d "D" same-rank=::a
}"#,
    )
    .unwrap();
    assert_eq!(rect(&c, "a").y, rect(&c, "d").y);
}

#[test]
fn hints_work_inside_containers() {
    let c = compile(
        r#"diagram main "Hints" type=graph layout=auto {
  node group {
    node result after=::group.input
    node input
  }
}"#,
    )
    .unwrap();
    assert!(rect(&c, "result").y > rect(&c, "input").bottom());
}

#[test]
fn invalid_hints_report_source_lines() {
    let cases = [
        (
            r#"diagram main "Hints" type=graph {
  node a after=::b
  node b
}"#,
            "require layout=auto",
        ),
        (
            r#"diagram main "Hints" type=graph layout=auto {
  node a after=missing
}"#,
            "unknown object reference",
        ),
        (
            r#"diagram main "Hints" type=graph layout=auto {
  node a after=::a
}"#,
            "own node",
        ),
        (
            r#"diagram main "Hints" type=graph layout=auto {
  node a after=::b
  node b after=::a
}"#,
            "below hint conflicts",
        ),
        (
            r#"diagram main "Hints" type=graph layout=auto {
  node a after=::b same-rank=::b
  node b
}"#,
            "below hint conflicts",
        ),
        (
            r#"diagram main "Hints" type=graph layout=auto {
  node a after=::b
  node b
  ::a -> ::b
}"#,
            "below hint conflicts",
        ),
        (
            r#"diagram main "Hints" type=graph layout=auto {
  node a beside=::b
  node b beside=::a
}"#,
            "beside hints form a cycle",
        ),
        (
            r#"diagram main "Hints" type=graph layout=auto {
  node a
  node b beside=::a
  node c beside=::a
}"#,
            "conflicting beside",
        ),
        (
            r#"diagram main "Hints" type=graph layout=auto {
  row {
    node a after=::b
    node b
  }
}"#,
            "explicit row cells",
        ),
        (
            r#"diagram main "Hints" type=graph layout=auto {
  node a after=::b
  section "Boundary" {
    node b
  }
}"#,
            "same automatic-layout region",
        ),
    ];
    for (src, message) in cases {
        let err = match compile(src) {
            Ok(_) => panic!("accepted invalid hint: {src}"),
            Err(err) => err,
        };
        assert!(err.line.is_some_and(|line| line >= 2));
        assert!(err.msg.contains(message), "{src}: {err}");
    }
}

#[test]
fn same_layer_across_a_path_has_documented_cycle_behavior() {
    let c = compile(
        r#"diagram main "Hints" type=graph layout=auto {
  node a "A"
  node b "B"
  node c "C" same-rank=::a
  ::a -> ::b
  ::b -> ::c
}"#,
    )
    .unwrap();
    assert_eq!(rect(&c, "a").y, rect(&c, "b").y);
    assert_eq!(rect(&c, "a").y, rect(&c, "c").y);
}
