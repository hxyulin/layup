use layup::{
    Theme, compile,
    document::{Color, StrokeStyle},
    layout::Item,
};
#[test]
fn declared_and_instance_styles_override_defaults_without_losing_inherited_paint() {
    let c = compile(
        r##"diagram main type=graph layout=auto {
      defaults node palette=gray
      defaults edge palette=gray stroke-style=dashed
      node-style parent fill-color="#abcdef"
      node-style service base=parent palette=blue font-family=mono
      edge-style calls palette=purple stroke-style=solid stroke-color="#543210"
      node a style=service
      node b style=service palette=green
      node plain
      edge call a -> b style=calls
    }"##,
    )
    .unwrap();
    let info = c.document.as_ref().unwrap();
    assert_eq!(c.scene.nodes[0].tone, layup::style::Tone::Blue);
    assert_eq!(c.scene.nodes[1].tone, layup::style::Tone::Green);
    assert_eq!(c.scene.nodes[2].tone, layup::style::Tone::Gray);
    assert_eq!(
        info.objects["object:[\"a\"]"].paint.fill_color,
        Some(Color::Literal("#abcdef".into()))
    );
    assert_eq!(
        info.objects["object:[\"b\"]"].paint.fill_color,
        Some(Color::Literal("#abcdef".into()))
    );
    assert_eq!(c.scene.edges[0].tone, layup::style::Tone::Purple);
    assert!(!c.scene.edges[0].dashed);
    assert_eq!(
        info.relationships["relationship:\"call\""]
            .paint
            .stroke_style,
        Some(StrokeStyle::Solid)
    );
}
#[test]
fn paint_changes_keep_geometry_and_routing_and_survive_scene_and_svg_exports() {
    let source =
        "diagram main type=graph layout=auto { node a; node b; edge call a -> b \"Call\" }";
    let before = compile(source).unwrap();
    let source = source.replace("node a", "node a fill-color={light:\"#abcdef\",dark:\"#123456\"} text-color=white stroke-style=dotted stroke-width=2.5")
        .replace("edge call a -> b \"Call\"", "edge call a -> b \"Call\" stroke-color={light:\"#fedcba\",dark:\"#654321\"}");
    let after = compile(&source).unwrap();
    assert_eq!(
        before
            .scene
            .nodes
            .iter()
            .map(|n| n.rect)
            .collect::<Vec<_>>(),
        after.scene.nodes.iter().map(|n| n.rect).collect::<Vec<_>>()
    );
    assert_eq!(before.scene.edges[0].points, after.scene.edges[0].points);
    let svg = layup::svg::render(&after, Theme::Auto);
    for fragment in [
        "#abcdef",
        "#123456",
        "#fedcba",
        "#654321",
        "stroke-dasharray:1 4",
        "stroke-width:2.5",
        "@media(prefers-color-scheme:dark)",
        "fill:var(--paint-edge-0-stroke)",
    ] {
        assert!(svg.contains(fragment), "{fragment}");
    }
    let scene: serde_json::Value =
        serde_json::from_str(&layup::scene::export(&after).unwrap()).unwrap();
    assert_eq!(scene["nodes"][0]["paint"]["textColor"]["type"], "literal");
    assert_eq!(scene["nodes"][0]["paint"]["textColor"]["value"], "white");
    assert_eq!(scene["edges"][0]["paint"]["strokeColor"]["type"], "themed");
    assert!(layup::html::render(&after, Theme::Auto, false).contains("--paint-edge-0-stroke"));
}
#[test]
fn paint_auto_resets_an_inherited_channel_and_containers_do_not_color_children() {
    let c = compile(
        r##"diagram main type=graph {
      node-style colored fill-color="#abc"
      group parent fill-color="#def" {
        node child style=colored fill-color=auto
        node plain
      }
    }"##,
    )
    .unwrap();
    let objects = &c.document.unwrap().objects;
    assert_eq!(
        objects["object:[\"parent\"]"].paint.fill_color,
        Some(Color::Literal("#def".into()))
    );
    assert_eq!(
        objects["object:[\"parent\",\"child\"]"].paint.fill_color,
        None
    );
    assert_eq!(
        objects["object:[\"parent\",\"plain\"]"].paint.fill_color,
        None
    );
}
#[test]
fn caption_and_legend_labels_are_independent_of_palette_inheritance() {
    let c = compile(
        r#"diagram main type=graph layout=auto {
      node-style service palette=blue legend-label="Service node"
      node a style=service
      node b style=service palette=green
      edge-style calls palette=target default-label="Request" legend-label="RPC traffic"
      edge call a -> b style=calls label=style
      legend visibility=visible nodes=[service] edges=[calls]
    }"#,
    )
    .unwrap();
    assert_eq!(c.diagram.edges[0].label.as_deref(), Some("Request"));
    assert_eq!(
        c.diagram.arrows["calls"].label.as_deref(),
        Some("RPC traffic")
    );
    assert_eq!(c.scene.edges[0].tone, layup::style::Tone::Green);
    assert!(matches!(
        c.scene.edges[0].chip,
        Some(Item::Chip {
            tone: layup::style::Tone::Green,
            ..
        })
    ));
    let texts = c
        .scene
        .items
        .iter()
        .filter_map(|p| {
            if let Item::Text(t) = &p.item {
                Some(t.runs.iter().map(|r| r.text.as_str()).collect::<String>())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(
        texts
            .iter()
            .filter(|text| text.as_str() == "Service node")
            .count(),
        1
    );
    assert!(!c.diagram.kinds.keys().any(|k| k.starts_with("__layup_")));
}
#[test]
fn rectangle_shape_and_quoted_style_names_have_safe_svg_representation() {
    let c = compile("diagram main type=graph { node-style \"unusual edge style\" shape=rectangle; node a style=\"unusual edge style\" }").unwrap();
    assert!(
        c.scene
            .items
            .iter()
            .any(|p| matches!(p.item, Item::Box { rx: 0., .. }))
    );
    let svg = layup::svg::render(&c, Theme::Light);
    assert!(svg.contains("k-unusual_20_edge_20_style"));
    assert!(!svg.contains("class=\"node k-unusual edge style"));
    assert_eq!(c.scene.nodes[0].kind, "unusual edge style");
}
#[test]
fn unsupported_paint_and_noop_properties_are_errors() {
    for source in [
        "diagram main type=graph { node a background-color=blue }",
        "diagram main type=graph style=service { node a }",
        "diagram main type=graph fill-color=blue { node a }",
        "diagram main type=graph { node a; node b; a -> b default-label=ignored }",
        "diagram main type=graph { defaults edge default-label=ignored; node a }",
        "diagram main type=graph { edge-style calls label=ignored; node a }",
        "diagram main type=graph { node a; view wrong participant-direction=left { include a } }",
        "diagram main type=sequence { participant a; view wrong flow-direction=left { include a } }",
        "diagram main type=graph { node a text-color=none }",
        "diagram main type=graph { node a fill-color=\"url(example)\" }",
        "diagram main type=graph { gap size=large }",
        "diagram main type=graph { gap \"Title\" size=20 }",
        "diagram main type=graph { row { node a; gap size=20 } }",
        "diagram main type=sequence { participant a after=a }",
        "diagram main type=graph { node a stroke-style=dot }",
        "diagram main type=state-machine { initial start style=process; final end; start -> end }",
        "diagram main type=graph { defaults edge type=reply; node a }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}
#[test]
fn selecting_a_view_cannot_hide_invalid_presentation_state_in_another_view() {
    let source = "diagram main type=graph { node a; view good { include a; step reveal { show objects=[a] } }; view bad { include a; step hidden { highlight objects=[a] } } }";
    let error = layup::compile_with_options(
        source,
        &layup::CompileOptions {
            view: Some("good".into()),
            ..Default::default()
        },
        &Default::default(),
    )
    .err()
    .unwrap();
    assert_eq!(error.code, "presentation/hidden-highlight");
}

#[test]
fn automatic_legend_position_does_not_force_unused_entries() {
    let c = compile("diagram main type=graph { node a; legend visibility=auto position=bottom }")
        .unwrap();
    assert!(c.diagram.legend.is_none());
    let c = compile("diagram main type=graph { node a; node b; a -> b style=uses; legend visibility=auto position=bottom }").unwrap();
    assert!(c.diagram.legend.unwrap().bottom);
    let c =
        compile("diagram main type=graph { node a; legend visibility=visible position=bottom }")
            .unwrap();
    assert!(c.diagram.legend.is_some());
}

#[test]
fn legend_samples_retain_declared_paint_and_merge_identical_literal_strokes() {
    let c = compile(include_str!("../../../examples/paint.layup")).unwrap();
    let groups = c
        .scene
        .items
        .iter()
        .filter_map(|placed| {
            if let Item::StyledGroup {
                edge, paint, items, ..
            } = &placed.item
            {
                Some((*edge, paint, items))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(groups.len(), 3);
    assert_eq!(groups.iter().filter(|(edge, _, _)| *edge).count(), 1);
    let (_, paint, _) = groups.iter().find(|(edge, _, _)| *edge).unwrap();
    assert_eq!(paint.stroke_style, Some(StrokeStyle::Dotted));
    let svg = layup::svg::render(&c, Theme::Auto);
    assert!(svg.contains("p-legend-edge-calls-gray"));
    assert!(svg.contains("--paint-legend-edge-calls-gray-stroke:#1d4ed8"));
    let json: serde_json::Value = serde_json::from_str(&layup::scene::export(&c).unwrap()).unwrap();
    assert_eq!(
        json["items"]
            .as_array()
            .unwrap()
            .iter()
            .filter(
                |item| item["drawing"]["type"] == "group" && item["drawing"].get("paint").is_some()
            )
            .count(),
        3
    );
}
