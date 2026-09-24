use layup::svg::{Options, render, render_with};
use layup::{Theme, compile};

#[test]
fn linked_nodes_wrap_their_items_in_an_anchor() {
    let c = compile(r#"diagram "T" { node a "A" href="guide/a?x=1&y=2"; node b "B" }"#).unwrap();
    let svg = render(&c, Theme::Light);
    let a = svg.find(r#"data-id="a""#).unwrap();
    let b = svg.find(r#"data-id="b""#).unwrap();
    let link = svg.find(r#"<a href="guide/a?x=1&amp;y=2">"#).unwrap();
    assert!(a < link && link < b);
    assert_eq!(svg.matches("<a ").count(), svg.matches("</a>").count());
}

#[test]
fn inline_diagrams_do_not_share_ids() {
    let svg = |title: &str| {
        let src = format!(r#"diagram "{title}" {{ node a; node b; a -> b }}"#);
        render(&compile(&src).unwrap(), Theme::Light)
    };
    let (one, two) = (svg("One"), svg("Two"));
    let prefix = |svg: &str| {
        svg.split(r#"<title id=""#)
            .nth(1)
            .unwrap()
            .split("title\"")
            .next()
            .unwrap()
            .to_string()
    };
    assert_ne!(prefix(&one), prefix(&two));
    for svg in [&one, &two] {
        let p = prefix(svg);
        assert!(svg.contains(&format!(r#"<marker id="{p}m-"#)));
        assert!(svg.contains(&format!("url(#{p}m-")));
        assert!(!svg.contains("url(#m-"));
    }
}

#[test]
fn dark_selector_replaces_the_media_query() {
    let c = compile(r#"diagram "T" { node a }"#).unwrap();
    let auto = render(&c, Theme::Auto);
    assert!(auto.contains("@media (prefers-color-scheme: dark)"));
    let class = render_with(
        &c,
        &Options {
            theme: Theme::Auto,
            dark_selector: Some(".dark"),
        },
    );
    assert!(class.contains(":is(.dark) .layup.auto{") && !class.contains("@media"));
}
