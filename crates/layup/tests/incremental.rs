use layup::layout::NodeRect;
use layup::{Compiled, compile};

const BASE: &str = r#"diagram "Service" layout=auto {
    node input "Input"
    node worker "Worker"
    node store "Store"
    input -> worker
    worker -> store
}"#;

fn node<'a>(c: &'a Compiled, id: &str) -> &'a NodeRect {
    &c.scene.nodes[c.scene.node(id).unwrap()]
}

fn existing_unchanged(before: &Compiled, after: &Compiled) {
    for id in ["input", "worker", "store"] {
        assert_eq!(node(before, id).rect, node(after, id).rect, "{id}");
        assert_eq!(node(before, id).tone, node(after, id).tone, "{id}");
    }
}

#[test]
fn unrelated_insertion_preserves_existing_geometry_colors_and_routes() {
    let before = compile(BASE).unwrap();
    for marker in ["    node input", "    node worker", "    node store"] {
        let after =
            compile(&BASE.replace(marker, &format!("    node notes \"Notes\"\n{marker}"))).unwrap();
        existing_unchanged(&before, &after);
        for (a, b) in before.scene.edges.iter().zip(&after.scene.edges) {
            assert_eq!(a.points, b.points);
        }
        assert!(node(&after, "notes").rect.y > node(&after, "store").rect.bottom());
        assert!(after.warnings.is_empty());
    }
}

#[test]
fn extending_a_chain_only_adds_space_below_it() {
    let before = compile(BASE).unwrap();
    let after = compile(&BASE.replace(
        "    input -> worker",
        "    node output\n    store -> output\n    input -> worker",
    ))
    .unwrap();
    existing_unchanged(&before, &after);
    assert!(after.scene.height > before.scene.height);
}

#[test]
fn appending_an_independent_subsystem_does_not_mix_layers() {
    let before = compile(BASE).unwrap();
    let after = compile(&BASE.replace(
        "    input -> worker",
        "    node metrics\n    node archive\n    metrics -> archive\n    input -> worker",
    ))
    .unwrap();
    existing_unchanged(&before, &after);
    assert!(node(&after, "metrics").rect.y > node(&after, "store").rect.bottom());
    assert!(node(&after, "archive").rect.y > node(&after, "metrics").rect.bottom());
}

#[test]
fn peer_growth_preserves_order_and_identity_while_reflowing() {
    let before = compile(BASE).unwrap();
    let after = compile(
        &BASE
            .replace("    node store", "    node audit\n    node store")
            .replace(
                "    worker -> store",
                "    input -> audit\n    audit -> store\n    worker -> store",
            ),
    )
    .unwrap();
    assert_eq!(node(&after, "worker").rect.y, node(&after, "audit").rect.y);
    assert!(node(&after, "worker").rect.x < node(&after, "audit").rect.x);
    assert!(node(&after, "worker").rect.w < node(&before, "worker").rect.w);
    for id in ["input", "worker", "store"] {
        assert_eq!(node(&before, id).tone, node(&after, id).tone);
    }
}

#[test]
fn edge_statement_order_does_not_change_node_layout() {
    let before = compile(BASE).unwrap();
    let after = compile(&BASE.replace(
        "input -> worker\n    worker -> store",
        "worker -> store\n    input -> worker",
    ))
    .unwrap();
    existing_unchanged(&before, &after);
}

#[test]
fn explicit_colors_still_override_identity_colors() {
    let c = compile(&BASE.replace("node worker", "node worker red")).unwrap();
    assert_eq!(node(&c, "worker").tone, layup::style::Tone::Red);
}

#[test]
fn undirected_relationships_keep_peers_in_one_region() {
    let c = compile(
        r#"diagram "Peers" layout=auto {
        node a; node b; node c; node d; a -- b; c -> d
    }"#,
    )
    .unwrap();
    assert_eq!(node(&c, "a").rect.y, node(&c, "b").rect.y);
    assert!(node(&c, "c").rect.y > node(&c, "a").rect.bottom());
}
