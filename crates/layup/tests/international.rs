mod support;
use layup::{
    Theme, compile,
    layout::Item,
    text::{self, Direction, Font},
};

fn flattened(lines: &[Vec<text::Run>]) -> Vec<String> {
    lines
        .iter()
        .map(|line| line.iter().map(|r| r.text.as_str()).collect())
        .collect()
}

#[test]
fn cjk_wraps_without_spaces_and_preserves_punctuation() {
    for source in [
        "這是一段中文說明，應該在合適的位置換行。",
        "日本語の説明文です。句読点も正しく配置します。",
        "한국어문자레이아웃을확인합니다.",
    ] {
        let lines = text::wrap(source, Font::Sans, 12.0, 72.0);
        let strings = flattened(&lines);
        assert!(lines.len() > 1, "{strings:?}");
        assert_eq!(strings.concat(), source);
        for line in &lines {
            assert!(text::runs_width(line, Font::Sans, 12.0) <= 72.1, "{line:?}");
        }
        for line in &strings {
            assert!(!line.starts_with(['，', '。', '、', '）']), "{line}");
            assert!(!line.ends_with(['（', '「']), "{line}");
        }
    }
}

#[test]
fn wraps_unicode_whitespace_and_hard_breaks_without_breaking_code() {
    assert_eq!(
        flattened(&text::wrap("one\ttwo\nthree", Font::Sans, 12.0, 500.0)),
        ["one two", "three"]
    );
    assert_eq!(
        flattened(&text::wrap("one\n\nthree", Font::Sans, 12.0, 500.0)),
        ["one", "", "three"]
    );
    let source = "中文 `GET /items` 後續文字";
    let lines = text::wrap(source, Font::Sans, 12.0, 65.0);
    assert!(
        lines
            .iter()
            .flatten()
            .any(|r| r.code && r.text == "GET /items")
    );
    assert_eq!(
        flattened(&text::wrap("a\u{a0}b", Font::Sans, 12.0, 1.0)),
        ["a\u{a0}b"]
    );
    let family = "👩‍👩‍👧‍👦";
    assert_eq!(
        flattened(&text::wrap(family, Font::Sans, 12.0, 1.0)),
        [family]
    );
}

#[test]
fn shaping_handles_combining_marks_and_rtl_fonts() {
    for font in [Font::Sans, Font::SansBold, Font::Mono] {
        assert!(
            (text::width("é", font, 12.0, 0.0) - text::width("e\u{301}", font, 12.0, 0.0)).abs()
                < 0.01
        );
        for c in "العربيةשָׁלוֹםé".chars() {
            assert!(text::supports(c, font), "{c} {font:?}");
        }
    }
    assert_eq!(
        text::width("\u{200d}\u{200e}\u{2066}\u{2069}", Font::Sans, 12.0, 0.0),
        0.0
    );
    let arabic = text::width("سلام", Font::Sans, 16.0, 0.0);
    let separate: f64 = "سلام"
        .chars()
        .map(|c| text::width(&c.to_string(), Font::Sans, 16.0, 0.0))
        .sum();
    assert!(
        arabic < separate,
        "Arabic joining and required ligatures must change advances"
    );
    assert_eq!(Direction::Auto.resolve("123 שלום API"), Direction::Rtl);
    assert_eq!(Direction::Auto.resolve("API العربية"), Direction::Ltr);
}

#[test]
fn multilingual_nodes_render_with_bundled_fonts_and_independent_direction() {
    let c = compile(
        r#"diagram main "國際文字" type=graph layout=auto flow-direction=right {
  node 中文 "資料輸入" {
    text "這是一段沒有空格的中文說明，應該正確換行。"
  }
  node عربي "معالجة الطلب" {
    text "يرسل الطلب إلى `GET /items` ثم يعيد النتيجة."
  }
  node עברית "תוצאה" {
    text "日本語と한국어も表示します。𠮷"
  }
  ::中文 -> ::عربي
  ::عربي -> ::עברית
}"#,
    )
    .unwrap();
    assert!(c.warnings.is_empty(), "{:?}", c.warnings);
    let svg = layup::svg::render(&c, Theme::Light);
    assert!(!svg.contains("font-family:'Layup CJK'"));
    assert!(svg.contains("font-family:'Layup Arabic'"));
    assert!(svg.contains("font-family:'Layup Hebrew'"));
    assert!(svg.contains("direction=\"rtl\""));
    assert!(svg.contains("direction=\"ltr\" unicode-bidi=\"isolate\">GET"));
    assert!(svg.contains("𠮷"));
    let id = support::node(&c.scene, "عربي").unwrap();
    let rect = c.scene.nodes[id].rect;
    for item in c.scene.items.iter().filter(|p| p.node == Some(id)) {
        if let Item::Text(t) = &item.item {
            assert_eq!(t.direction, Direction::Rtl);
            assert!(t.x > rect.cx(), "RTL labels align at the right padding");
        }
    }
}

#[test]
fn node_direction_overrides_inherit_through_containers() {
    let c = compile(
        r#"diagram main "T" type=graph text-direction=rtl {
  node outer "Outer" text-direction=ltr {
    node inner "123 API" {
      text "456 API"
    }
  }
  node rtl "123 API"
}"#,
    )
    .unwrap();
    for (id, expected) in [
        ("outer", Direction::Ltr),
        ("inner", Direction::Ltr),
        ("rtl", Direction::Rtl),
    ] {
        let idx = support::node(&c.scene, id).unwrap();
        for p in c.scene.items.iter().filter(|p| p.node == Some(idx)) {
            if let Item::Text(t) = &p.item {
                assert_eq!(t.direction, expected, "{id}");
            }
        }
    }
    assert!(
        compile(
            r#"diagram main "T" type=graph text-direction=sideways {
}"#
        )
        .is_err()
    );
    assert!(
        compile(
            r#"diagram main "T" type=graph {
  node n text-direction=sideways
}"#
        )
        .is_err()
    );
}

#[test]
fn unknown_glyphs_warn_instead_of_claiming_deterministic_metrics() {
    let c = compile("diagram main \"T\" type=graph {\n  node n \"🦄🦄\"\n}").unwrap();
    let warnings: Vec<_> = c
        .warnings
        .iter()
        .filter(|w| w.msg.contains("U+1F984"))
        .collect();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].line, Some(2));
}

#[test]
fn decomposed_identifiers_can_be_referenced() {
    let c = compile(
        "diagram main \"T\" type=graph { node cafe\u{301} \"Café\"; node 结果 \"結果\"; cafe\u{301} -> 结果 }",
    )
    .unwrap();
    assert!(support::node(&c.scene, "cafe\u{301}").is_some());
    assert!(c.warnings.is_empty(), "{:?}", c.warnings);
}

#[test]
fn supplied_font_bytes_drive_measurement_wrapping_and_embedding() {
    let bytes = include_bytes!("fonts/Fallback.ttf");
    let mut fonts = text::Fonts::new();
    assert!(!text::supports('中', Font::Sans));
    fonts.add_fallback(bytes.as_slice()).unwrap();
    assert!(text::supports_with_fonts('中', Font::Sans, &fonts));
    let measured = text::width_with_fonts("中", Font::Sans, 12.0, 0.0, &fonts);
    assert!((measured - text::width("M", Font::Sans, 12.0, 0.0)).abs() < 0.01);
    assert!((measured - text::width("中", Font::Sans, 12.0, 0.0)).abs() > 0.1);
    let c = layup::compile_with_fonts(
        r#"diagram main "中" type=graph width=200 {
  node n "中中中" {
    text "中中中中中中中中中中中中中中中中中中中中"
  }
}"#,
        &fonts,
    )
    .unwrap();
    assert!(c.warnings.is_empty(), "{:?}", c.warnings);
    assert!(layup::svg::render(&c, Theme::Light).contains("font-family:'Layup User "));
    assert!(fonts.add_fallback(vec![0, 1, 2]).is_err());
    let default = compile(
        r#"diagram main "中" type=graph {
  node n "中"
}"#,
    )
    .unwrap();
    assert!(!layup::svg::render(&default, Theme::Light).contains("font-family:'Layup User "));
}
