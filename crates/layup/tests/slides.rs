use layup::{
    compile,
    layout::Scene,
    parser::{Item, Stmt, parse},
    slides::{Slide, SlidePlan},
};

fn root(source: &str) -> Item {
    let Stmt::Item(root) = parse(source).unwrap().remove(0) else {
        panic!("expected an item");
    };
    root
}

fn slide(attributes: &str) -> Slide {
    Slide::take(&mut root(&format!("diagram \"Slide\" {attributes} {{}}")))
        .unwrap()
        .unwrap()
}

fn assert_fits(plan: &SlidePlan, scene: &Scene) {
    let epsilon = 1e-7;
    let right = plan.offset_x + scene.width * plan.scale;
    let bottom = plan.offset_y + scene.height * plan.scale;
    assert!(plan.scale > 0.0 && plan.scale.is_finite());
    assert!(plan.offset_x >= plan.padding - epsilon);
    assert!(plan.offset_y >= plan.padding - epsilon);
    assert!(right <= plan.width - plan.padding + epsilon);
    assert!(bottom <= plan.height - plan.padding + epsilon);
    assert!((plan.offset_x - (plan.width - right)).abs() < epsilon);
    assert!((plan.offset_y - (plan.height - bottom)).abs() < epsilon);
    for &(x, y) in scene.edges.iter().flat_map(|e| &e.points) {
        let x = plan.offset_x + x * plan.scale;
        let y = plan.offset_y + y * plan.scale;
        assert!(x >= 0.0 && x <= plan.width);
        assert!(y >= 0.0 && y <= plan.height);
    }
}

#[test]
fn viewport_presets_custom_dimensions_and_attribute_extraction() {
    for (value, width, height) in [
        ("wide", 1920.0, 1080.0),
        ("standard", 1440.0, 1080.0),
        ("\"16:9\"", 1920.0, 1080.0),
        ("\"4:3\"", 1440.0, 1080.0),
        ("\"1280:720\"", 1280.0, 720.0),
        ("\"1080:1920\"", 1080.0, 1920.0),
    ] {
        let mut item = root(&format!(
            "diagram \"中文\" slide={value} layout=auto slide-padding=30 min-font-size=20 width=960 {{ node a }}"
        ));
        let previous = item.arg_spans.clone();
        let slide = Slide::take(&mut item).unwrap().unwrap();
        assert_eq!((slide.width, slide.height), (width, height));
        assert_eq!((slide.padding, slide.min_font_size), (30.0, 20.0));
        assert_eq!(item.args.len(), 3);
        assert_eq!(item.arg_spans, vec![previous[0], previous[2], previous[5]]);
        assert!(Slide::take(&mut item).unwrap().is_none());
        assert_eq!(item.body.unwrap().len(), 1);
    }
}

#[test]
fn invalid_slide_arguments_have_precise_positions_and_preserve_ast() {
    for attribute in [
        "slide=tiny",
        "slide=1920",
        "slide=\"0:1080\"",
        "slide=\"1920:-1\"",
        "slide=\"NaN:1080\"",
        "slide=\"1920:inf\"",
        "slide=\"1920:1080:1\"",
        "slide-padding=20",
        "min-font-size=20",
        "slide=wide slide-padding=-1",
        "slide=wide slide-padding=540",
        "slide=wide slide-padding=\"48\"",
        "slide=wide min-font-size=0",
        "slide=wide min-font-size=-20",
        "slide=wide min-font-size=large",
    ] {
        let source = format!("// 中文\ndiagram \"中文\" {attribute} {{ node a }}");
        let mut item = root(&source);
        let old_args = item.args.clone();
        let old_spans = item.arg_spans.clone();
        let error = Slide::take(&mut item).unwrap_err();
        assert_eq!(error.code, "semantic/slide");
        let span = *error.span.unwrap();
        assert_eq!(span.line, 2, "{attribute}");
        assert!(source[span.start..span.end].contains('='), "{attribute}");
        assert_eq!(item.args, old_args);
        assert_eq!(item.arg_spans, old_spans);
    }
}

#[test]
fn routed_graph_fits_both_landscape_and_portrait_with_letterboxing() {
    let c = compile(
        "diagram \"Workflow\" layout=auto { decision a \"Ready?\"; terminal b \"Done\"; process d \"Prepare\"; a -> b \"Yes\"; a -> d \"No\"; d -> a }",
    )
    .unwrap();
    for attributes in [
        "slide=wide",
        "slide=standard",
        "slide=\"1080:1920\" slide-padding=80",
        "slide=\"1000:1000\" slide-padding=0",
    ] {
        let plan = slide(attributes).plan(&c.scene, &mut Vec::new());
        assert_fits(&plan, &c.scene);
        assert_eq!(plan.content_width, c.scene.width);
        assert_eq!(plan.content_height, c.scene.height);
    }
}

#[test]
fn long_diagrams_report_effective_readability_without_reflow() {
    let mut source = String::from(
        "diagram \"Many steps\" mode=state-machine direction=right { initial start; final end; start -> q0;",
    );
    for i in 0..18 {
        source.push_str(&format!("state q{i} \"Processing step {i}\";"));
        if i > 0 {
            source.push_str(&format!("q{} -> q{i} \"next\";", i - 1));
        }
    }
    source.push_str("q17 -> end }");
    let c = compile(&source).unwrap();
    let mut warnings = Vec::new();
    let plan = slide("slide=wide min-font-size=24").plan(&c.scene, &mut warnings);
    assert_fits(&plan, &c.scene);
    assert!(plan.scale < 1.0);
    assert!(plan.smallest_font_size.unwrap() < 24.0);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].msg.contains("slide readability"));
    assert!(warnings[0].msg.contains("split the diagram into views"));
    assert_eq!(warnings[0].line, Some(1));
    let mut warnings = Vec::new();
    slide("slide=wide min-font-size=1").plan(&c.scene, &mut warnings);
    assert!(warnings.is_empty());
}

#[test]
fn label_chips_participate_in_readability_checks() {
    for (spacing, expected_size) in [("", 12.0), ("gap 200;", 11.5)] {
        let c = compile(&format!(
            "diagram \"Messages\" preset=manual width=900 {{ node a; {spacing} node b; a -> b \"payload\" }}"
        ))
        .unwrap();
        let options = slide(&format!(
            "slide=\"900:2000\" slide-padding=0 min-font-size={expected_size}"
        ));
        let mut warnings = Vec::new();
        let plan = options.plan(&c.scene, &mut warnings);
        assert_eq!(plan.scale, 1.0);
        assert_eq!(plan.smallest_font_size, Some(expected_size));
        assert!(warnings.is_empty());
        let options = slide("slide=\"900:2000\" slide-padding=0 min-font-size=12.5");
        options.plan(&c.scene, &mut warnings);
        assert_eq!(warnings.len(), 1);
    }
}

#[test]
fn supplied_fonts_change_fit_through_the_measured_scene() {
    let mut source = String::from(
        "diagram \"Fonts\" mode=state-machine direction=right { initial start; final end; start -> q0;",
    );
    for i in 0..10 {
        source.push_str(&format!("state q{i} \"中中中中中中中中\";"));
        if i > 0 {
            source.push_str(&format!("q{} -> q{i};", i - 1));
        }
    }
    source.push_str("q9 -> end }");
    let default = compile(&source).unwrap();
    let mut fonts = layup::text::Fonts::new();
    fonts
        .add_fallback(include_bytes!("fonts/Fallback.ttf").as_slice())
        .unwrap();
    let supplied = layup::compile_with_fonts(&source, &fonts).unwrap();
    let options = slide("slide=wide min-font-size=1");
    let default_plan = options.plan(&default.scene, &mut Vec::new());
    let supplied_plan = options.plan(&supplied.scene, &mut Vec::new());
    assert!(supplied_plan.scale > default_plan.scale);
    assert_fits(&default_plan, &default.scene);
    assert_fits(&supplied_plan, &supplied.scene);
}

#[test]
fn compilation_preserves_geometry_and_renders_the_slide_viewport() {
    let source = "diagram \"Keep layout\" layout=auto { process a \"Request\"; process b \"Response\"; a -> b \"Payload\" }";
    let original = compile(source).unwrap();
    assert!(original.slide.is_none());
    let fitted =
        compile(&source.replace("layout=auto", "layout=auto slide=wide min-font-size=1")).unwrap();
    let plan = fitted.slide.as_ref().unwrap();
    assert_eq!(fitted.viewport(), (1920.0, 1080.0));
    assert_eq!(fitted.scene.width, original.scene.width);
    assert_eq!(fitted.scene.height, original.scene.height);
    for (before, after) in original.scene.nodes.iter().zip(&fitted.scene.nodes) {
        assert_eq!(before.rect, after.rect);
    }
    for (before, after) in original.scene.edges.iter().zip(&fitted.scene.edges) {
        assert_eq!(before.points, after.points);
    }
    assert_fits(plan, &fitted.scene);
    let svg = layup::svg::render(&fitted, layup::Theme::Light);
    assert!(svg.contains("viewBox=\"0 0 1920 1080\""));
    assert!(svg.contains("class=\"diagram-transform\" transform="));
    let html = layup::html::render(&fitted, layup::Theme::Light, false);
    assert!(html.contains("viewBox=\"0 0 1920 1080\""));
}

#[test]
fn nested_message_label_groups_participate_in_readability_checks() {
    use layup::layout::{Item as DrawItem, Placed};
    let mut c = compile("diagram \"Groups\" preset=manual width=900 { node a }").unwrap();
    let mut text = c
        .scene
        .items
        .iter()
        .find_map(|p| match &p.item {
            DrawItem::Text(t) => Some(t.clone()),
            _ => None,
        })
        .unwrap();
    text.size = 5.0;
    c.scene.items.push(Placed {
        item: DrawItem::Group(vec![DrawItem::Group(vec![DrawItem::Text(text)])]),
        node: None,
    });
    let options = slide("slide=\"900:2000\" slide-padding=0 min-font-size=6");
    let mut warnings = Vec::new();
    let plan = options.plan(&c.scene, &mut warnings);
    assert_eq!(plan.smallest_font_size, Some(5.0));
    assert_eq!(warnings.len(), 1);
}

#[test]
fn finite_custom_viewports_keep_their_authored_numeric_dimensions() {
    for dimensions in ["0.001:0.002", "100000000000000000000:1080"] {
        let source = format!(
            "diagram \"Numeric viewport\" slide=\"{dimensions}\" slide-padding=0 min-font-size=1 {{ node a }}"
        );
        let c = compile(&source).unwrap();
        let slide = c.slide.as_ref().unwrap();
        assert!(slide.scale.is_finite() && slide.scale > 0.0);
        let svg = layup::svg::render(&c, layup::Theme::Light);
        let expected = format!("viewBox=\"0 0 {} {}\"", slide.width, slide.height);
        assert!(
            svg.contains(&expected),
            "custom dimensions were rounded or clamped: {dimensions}"
        );
    }
}
