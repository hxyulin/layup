use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::{Value as Json, json};

use super::{
    Annotation, Attributes, Body, ConfigurationKind, DefaultCategory, Document, Edge, FragmentKind,
    GraphStatement, Kind, LayoutKind, NamedBlock, Reference, SelectionKind, Value,
};
use crate::{
    CompileOptions, Compiled, Error,
    diagnostic::Span,
    input::SourceLocation,
    parser::{Arg, EdgeStmt, Item, Stmt, Value as OldValue},
    text::Fonts,
};

/// Authored identities and annotations, independent of opaque renderer IDs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub language_version: u32,
    pub diagram_id: String,
    pub diagrams: Vec<String>,
    pub manifest: Vec<ManifestEntry>,
    pub targets: Vec<TargetInfo>,
    pub diagnostics: Vec<Json>,
    pub diagram: EntityInfo,
    pub objects: BTreeMap<String, EntityInfo>,
    pub relationships: BTreeMap<String, EntityInfo>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityInfo {
    pub declaration: Option<String>,
    pub attributes: Attributes,
    pub path: Option<Vec<String>>,
    pub authored_id: Option<String>,
    pub documentation: Option<String>,
    pub source_locations: Vec<SourceLocation>,
    pub metadata: BTreeMap<String, Json>,
    pub annotations: Vec<Annotation>,
    pub paint: super::Paint,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestEntry {
    pub id: String,
    pub diagram_type: String,
    pub status: String,
    pub reason: Option<String>,
    pub annotations: Vec<Annotation>,
    pub span: Span,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetInfo {
    pub diagram_id: String,
    pub category: String,
    pub span: Span,
    pub annotations: Vec<Annotation>,
}
fn targets(document: &Document) -> Result<Vec<TargetInfo>, Error> {
    fn visit(
        body: &[GraphStatement],
        diagram: &str,
        output: &mut Vec<TargetInfo>,
    ) -> Result<(), Error> {
        for statement in body {
            let category = match statement {
                GraphStatement::Node(_) => "object",
                GraphStatement::Edge(_) => "relationship",
                GraphStatement::NodeStyle(_) | GraphStatement::EdgeStyle(_) => "style",
                GraphStatement::View(_) => "view",
                GraphStatement::Step(_) => "step",
                GraphStatement::Row { .. } | GraphStatement::Layout { .. } => "layout",
                GraphStatement::Configuration { .. } => "configuration",
                GraphStatement::Defaults { .. } => "defaults",
                GraphStatement::Selection { .. } => "selection",
                GraphStatement::Fragment { .. } => "fragment",
                GraphStatement::SequenceNote { .. } => "event",
                GraphStatement::Port { .. } => "member",
                GraphStatement::Content { .. } => "content",
            };
            let span = statement.span();
            let values = statement.annotations();
            if category != "object"
                && category != "relationship"
                && values.iter().any(|a| a.name == "source")
            {
                return Err(fail(
                    span,
                    "document/annotation",
                    "@source requires a diagram, object or relationship target",
                ));
            }
            annotations(values)?;
            if !values.is_empty() {
                output.push(TargetInfo {
                    diagram_id: diagram.into(),
                    category: category.into(),
                    span,
                    annotations: values.to_vec(),
                });
            }
            visit(statement.children(), diagram, output)?;
        }
        Ok(())
    }
    let mut output = Vec::new();
    for diagram in &document.diagrams {
        if let Some(body) = diagram.body.statements() {
            visit(body, &diagram.id, &mut output)?;
        }
    }
    Ok(output)
}

pub(super) fn validate_annotations(document: &Document) -> Result<(), Error> {
    for diagram in &document.diagrams {
        annotations(&diagram.annotations)?;
    }
    targets(document)?;
    Ok(())
}

pub(super) fn fail(span: Span, code: &'static str, message: impl Into<String>) -> Error {
    Error::located(span, code, message)
}
fn text(value: &Value, span: Span) -> Result<String, Error> {
    match value {
        Value::Choice(s) | Value::String(s) => Ok(s.clone()),
        _ => Err(fail(span, "document/value", "expected a word or string")),
    }
}
fn old(value: &Value, span: Span) -> Result<OldValue, Error> {
    match value {
        Value::Choice(s) => Ok(OldValue::Ident(s.clone())),
        Value::String(s) => Ok(OldValue::Str(s.clone())),
        Value::Integer(n) => Ok(OldValue::Num(n.as_f64())),
        Value::Float(n) => Ok(OldValue::Num(*n)),
        _ => Err(fail(
            span,
            "document/value",
            "this property requires a scalar word, string or number",
        )),
    }
}
// Source vocabulary maps onto the existing renderer while its geometry contract stays stable.
fn property(name: &str) -> &str {
    match name {
        "style" => "kind",
        "palette" => "tone",
        "font-family" => "font",
        "text-align" => "align",
        "text-direction" => "textdir",
        "stroke-style" => "stroke",
        "flow-direction" | "participant-direction" => "direction",
        "default-label" => "label",
        "legend-label" => "legend-label",
        "gap" => "gutter",
        "same-rank" => "same-layer",
        "route-side" => "via",
        other => other,
    }
}
fn source_property(name: &str) -> &str {
    match name {
        "kind" => "style",
        "tone" => "palette",
        "font" => "font-family",
        "align" => "text-align",
        "textdir" => "text-direction",
        "stroke" => "stroke-style",
        "direction" => "flow-direction",
        "label" => "label",
        "gutter" => "gap",
        "same-layer" => "same-rank",
        "via" => "route-side",
        other => other,
    }
}
fn allowed(attrs: &Attributes, names: &[&str]) -> Result<(), Error> {
    let mut used = BTreeSet::new();
    for (name, a) in attrs {
        if !used.insert(property(name)) {
            return Err(fail(
                a.span,
                "document/attribute",
                format!("property `{name}` is assigned more than once"),
            ));
        }
        let recognized = match name.as_str() {
            "default-label" | "label" => names.contains(&name.as_str()),
            _ => names.contains(&property(name)),
        };
        if !recognized
            && !(super::paint::is_paint(name)
                && (names.contains(&"tone") || names.contains(&"kind")))
        {
            return Err(fail(
                a.name_span,
                "document/attribute",
                format!("unsupported attribute `{name}` for this declaration"),
            )
            .with_optional_help(crate::diagnostic::suggestion(
                name,
                names.iter().map(|name| source_property(name)),
            )));
        }
    }
    Ok(())
}
fn direction_options(attributes: &Attributes, grammar: &str) -> Result<(), Error> {
    let (name, choices): (&str, &[&str]) = if grammar == "sequence" {
        ("participant-direction", &["right", "left"])
    } else {
        ("flow-direction", &["down", "up", "right", "left"])
    };
    for (key, attribute) in attributes {
        if property(key) == "direction"
            && (key != name
                || !choices.contains(&text(&attribute.value, attribute.value_span)?.as_str()))
        {
            return Err(fail(
                attribute.name_span,
                "document/direction",
                format!("use {name}={}", choices.join("|")),
            ));
        }
    }
    Ok(())
}
fn style_values(attrs: &Attributes, edge: bool) -> Result<(), Error> {
    for (name, attribute) in attrs {
        let choices: Option<&[&str]> = match name.as_str() {
            "palette" => Some(if edge {
                &[
                    "auto", "source", "target", "gray", "blue", "green", "yellow", "purple",
                    "orange", "red",
                ]
            } else {
                &[
                    "auto", "gray", "blue", "green", "yellow", "purple", "orange", "red",
                ]
            }),
            "font-family" => Some(&["mono", "sans"]),
            "text-align" => Some(&["start", "end", "left", "right", "center"]),
            "text-direction" => Some(&["auto", "ltr", "rtl"]),
            "shape" => Some(&["rectangle", "rounded-rectangle", "diamond", "capsule"]),
            "stroke-style" => Some(&["solid", "dashed", "dotted"]),
            _ => None,
        };
        if let Some(choices) = choices
            && !choices.contains(&text(&attribute.value, attribute.value_span)?.as_str())
        {
            return Err(fail(
                attribute.value_span,
                "document/value",
                format!("invalid {name}; choose {}", choices.join("|")),
            ));
        }
    }
    Ok(())
}
fn attr_text(attrs: &Attributes, name: &str) -> Result<Option<String>, Error> {
    attrs
        .iter()
        .find(|(key, _)| property(key) == name)
        .map(|(_, a)| text(&a.value, a.span))
        .transpose()
}
fn json_value(value: &Value, span: Span) -> Result<Json, Error> {
    Ok(match value {
        Value::Choice(s) | Value::String(s) => json!(s),
        Value::Integer(super::Integer::Signed(n)) => json!(n),
        Value::Integer(super::Integer::Unsigned(n)) => json!(n),
        Value::Float(n) => json!(n),
        Value::Bool(b) => json!(b),
        Value::Null => Json::Null,
        Value::List(values) => Json::Array(
            values
                .iter()
                .map(|v| json_value(v, span))
                .collect::<Result<_, _>>()?,
        ),
        Value::Record(values) => Json::Object(
            values
                .iter()
                .map(|(k, v)| Ok((k.clone(), json_value(v, span)?)))
                .collect::<Result<_, Error>>()?,
        ),
        Value::Reference(_) => {
            return Err(fail(
                span,
                "document/annotation",
                "annotation data cannot contain object references",
            ));
        }
    })
}
fn annotations(values: &[Annotation]) -> Result<EntityInfo, Error> {
    let mut info = EntityInfo {
        annotations: values.to_vec(),
        ..Default::default()
    };
    for annotation in values {
        let args = &annotation.arguments;
        let required = |key| {
            attr_text(args, key)?.ok_or_else(|| {
                fail(
                    annotation.span,
                    "document/annotation",
                    format!("@{} requires `{key}`", annotation.name),
                )
            })
        };
        match annotation.name.as_str() {
            "doc" => {
                allowed(args, &["text"])?;
                if info.documentation.is_some() {
                    return Err(fail(
                        annotation.span,
                        "document/annotation",
                        "@doc is not repeatable",
                    ));
                }
                info.documentation = Some(required("text")?);
            }
            "source" => {
                allowed(args, &["uri", "symbol", "range"])?;
                let uri = required("uri")?;
                if uri.is_empty() {
                    return Err(fail(
                        annotation.span,
                        "document/annotation",
                        "source URI cannot be empty",
                    ));
                }
                let range = args
                    .get("range")
                    .map(|a| -> Result<crate::input::SourceRange, Error> {
                        let range: crate::input::SourceRange = serde_json::from_value({
                            let mut value = json_value(&a.value, a.span)?;
                            if let Some(fields) = value.as_object_mut() {
                                if fields.keys().any(|key| !["start-line", "start-column", "end-line", "end-column"].contains(&key.as_str())) { return Err(fail(a.value_span, "document/annotation", "source ranges use start-line, start-column, end-line and end-column")); }
                                for (source, wire) in [
                                    ("start-line", "startLine"),
                                    ("start-column", "startColumn"),
                                    ("end-line", "endLine"),
                                    ("end-column", "endColumn"),
                                ] {
                                    if let Some(value) = fields.remove(source)
                                        && fields.insert(wire.into(), value).is_some()
                                    {
                                        return Err(fail(
                                            a.span,
                                            "document/annotation",
                                            format!("duplicate range property `{source}`"),
                                        ));
                                    }
                                }
                            }
                            value
                        })
                        .map_err(|e| {
                            fail(
                                a.span,
                                "document/annotation",
                                format!("invalid source range: {e}"),
                            )
                        })?;
                        if [
                            range.start_line,
                            range.start_column,
                            range.end_line,
                            range.end_column,
                        ]
                        .contains(&0)
                            || (range.end_line, range.end_column)
                                < (range.start_line, range.start_column)
                        {
                            return Err(fail(
                                a.span,
                                "document/annotation",
                                "source ranges use 1-based positions and an ordered exclusive end",
                            ));
                        }
                        Ok(range)
                    })
                    .transpose()?;
                info.source_locations.push(SourceLocation {
                    uri,
                    symbol: attr_text(args, "symbol")?,
                    range,
                });
            }
            "meta" => {
                allowed(args, &["namespace", "value"])?;
                let namespace = required("namespace")?;
                if namespace.is_empty() {
                    return Err(fail(
                        annotation.span,
                        "document/annotation",
                        "metadata namespace cannot be empty",
                    ));
                }
                let value = args.get("value").ok_or_else(|| {
                    fail(
                        annotation.span,
                        "document/annotation",
                        "@meta requires `value`",
                    )
                })?;
                if info
                    .metadata
                    .insert(namespace.clone(), json_value(&value.value, value.span)?)
                    .is_some()
                {
                    return Err(fail(
                        annotation.span,
                        "document/annotation",
                        format!("duplicate @meta namespace `{namespace}`"),
                    ));
                }
            }
            _ => {}
        }
    }
    Ok(info)
}
fn item(head: &str, args: Vec<Arg>, body: Option<Vec<Stmt>>, span: Span) -> Stmt {
    let arg_spans = vec![span; args.len()];
    Stmt::Item(Item {
        head: head.into(),
        args,
        body,
        line: span.line,
        span,
        head_span: span,
        arg_spans,
    })
}
fn attr(name: &str, value: impl Into<String>) -> Arg {
    Arg::Attr(name.into(), OldValue::Str(value.into()))
}
fn slide_args(attributes: &Attributes, span: Span) -> Result<Vec<Arg>, Error> {
    let mut args = Vec::new();
    allowed(attributes, &["size", "padding", "min-font-size"])?;
    let size = attributes
        .get("size")
        .ok_or_else(|| fail(span, "document/slide", "slide requires size"))?;
    let value = match &size.value {
        Value::Choice(s) | Value::String(s) if ["wide", "standard"].contains(&s.as_str()) => {
            s.clone()
        }
        Value::Record(r) if r.len() == 2 && r.contains_key("width") && r.contains_key("height") => {
            let num = |v: &Value| match v {
                Value::Integer(n) => Some(n.as_f64()),
                Value::Float(n) => Some(*n),
                _ => None,
            };
            let w = num(&r["width"])
                .filter(|n| *n > 0.)
                .ok_or_else(|| fail(size.span, "document/slide", "width must be positive"))?;
            let h = num(&r["height"])
                .filter(|n| *n > 0.)
                .ok_or_else(|| fail(size.span, "document/slide", "height must be positive"))?;
            format!("{w}:{h}")
        }
        _ => {
            return Err(fail(
                size.span,
                "document/slide",
                "size must be wide, standard or {width, height}",
            ));
        }
    };
    args.push(attr("slide", value));
    for (key, a) in attributes.iter().filter(|(k, _)| *k != "size") {
        args.push(Arg::Attr(
            if key == "padding" {
                "slide-padding".into()
            } else {
                key.clone()
            },
            old(&a.value, a.span)?,
        ));
    }
    Ok(args)
}
fn relationship_id(id: &str) -> String {
    format!("relationship:{}", serde_json::to_string(id).unwrap())
}
fn render_id(path: &[String]) -> String {
    format!(
        "object:{}",
        serde_json::to_string(path).expect("strings are serializable")
    )
}

struct Builder<'a> {
    paths: BTreeMap<Vec<String>, Span>,
    node_kinds: BTreeMap<String, &'a Kind>,
    edge_kinds: BTreeMap<String, &'a Kind>,
    styles: Vec<Stmt>,
    objects: BTreeMap<String, EntityInfo>,
    relationships: BTreeMap<String, EntityInfo>,
    edge_names: BTreeSet<String>,
    semantic_kinds: BTreeMap<String, String>,
    serial: usize,
    mode: String,
    defaults: BTreeMap<String, Attributes>,
}
impl<'a> Builder<'a> {
    fn collect(
        &mut self,
        body: &'a [GraphStatement],
        scope: &[String],
        diagram_level: bool,
    ) -> Result<(), Error> {
        for statement in body {
            match statement {
                GraphStatement::Node(node) => {
                    for end in 1..=scope.len() {
                        if scope[end - 1] == node.id
                            || self.paths.contains_key(
                                &[scope[..end - 1].to_vec(), vec![node.id.clone()]].concat(),
                            )
                        {
                            return Err(fail(
                                node.span,
                                "document/shadowing",
                                format!("`{}` shadows an ancestor-scope name", node.id),
                            ));
                        }
                    }
                    let path = [scope.to_vec(), vec![node.id.clone()]].concat();
                    if let Some(previous) = self.paths.insert(path.clone(), node.span) {
                        return Err(fail(
                            node.span,
                            "document/duplicate-id",
                            format!("duplicate object `{}` in this scope", node.id),
                        )
                        .with_related(previous, "first declaration"));
                    }
                    self.collect(&node.body, &path, false)?;
                }
                GraphStatement::Row { body, .. }
                | GraphStatement::Layout { body, .. }
                | GraphStatement::Fragment { body, .. } => self.collect(body, scope, false)?,
                GraphStatement::Edge(edge) => {
                    if let Some(id) = &edge.id
                        && !self.edge_names.insert(id.clone())
                    {
                        return Err(fail(
                            edge.span,
                            "document/duplicate-edge",
                            format!("duplicate relationship `{id}`"),
                        ));
                    }
                }
                GraphStatement::Defaults {
                    category,
                    attributes,
                    span,
                    ..
                } => {
                    if !diagram_level {
                        return Err(fail(
                            *span,
                            "document/defaults",
                            "defaults belong directly to the diagram",
                        ));
                    }
                    let category = match category {
                        DefaultCategory::Node => "node",
                        DefaultCategory::Edge => "edge",
                        DefaultCategory::Participant => "participant",
                        DefaultCategory::Message => "message",
                        DefaultCategory::State => "state",
                        DefaultCategory::Transition => "transition",
                    };
                    let valid = match self.mode.as_str() {
                        "sequence" => ["participant", "message"].contains(&category),
                        "state-machine" => ["state", "transition"].contains(&category),
                        _ => ["node", "edge"].contains(&category),
                    };
                    if !valid
                        || self
                            .defaults
                            .insert(category.into(), attributes.clone())
                            .is_some()
                    {
                        return Err(fail(
                            *span,
                            "document/defaults",
                            "invalid or duplicate defaults category",
                        ));
                    }
                }
                GraphStatement::NodeStyle(kind) | GraphStatement::EdgeStyle(kind) => {
                    if !diagram_level {
                        return Err(fail(
                            kind.span,
                            "document/kind",
                            "style declarations belong directly to the diagram",
                        ));
                    }
                    let kinds = if matches!(statement, GraphStatement::NodeStyle(_)) {
                        &mut self.node_kinds
                    } else {
                        &mut self.edge_kinds
                    };
                    let builtin = if matches!(statement, GraphStatement::NodeStyle(_)) {
                        crate::style::presets().contains_key(&kind.id)
                    } else {
                        crate::style::arrow_presets().contains_key(&kind.id)
                    };
                    if builtin || kinds.insert(kind.id.clone(), kind).is_some() {
                        return Err(fail(
                            kind.span,
                            "document/kind",
                            format!("duplicate or built-in style `{}`", kind.id),
                        ));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn validate_shadowing(&self) -> Result<(), Error> {
        // Do a second pass so forward declarations cannot change shadowing.
        for (path, span) in &self.paths {
            let name = path.last().unwrap();
            for end in 0..path.len().saturating_sub(1) {
                let ancestor = [path[..end].to_vec(), vec![name.clone()]].concat();
                if let Some(previous) = self.paths.get(&ancestor) {
                    return Err(fail(
                        *span,
                        "document/shadowing",
                        format!("`{name}` shadows an ancestor-scope name"),
                    )
                    .with_related(*previous, "ancestor declaration"));
                }
            }
        }
        Ok(())
    }
    fn resolve(
        &self,
        reference: &Reference,
        scope: &[String],
        span: Span,
    ) -> Result<String, Error> {
        let first = reference
            .segments
            .first()
            .ok_or_else(|| fail(span, "document/reference", "empty reference"))?;
        for end in (0..=if reference.root { 0 } else { scope.len() }).rev() {
            let prefix = [scope[..end].to_vec(), vec![first.clone()]].concat();
            if self.paths.contains_key(&prefix) {
                let full = [scope[..end].to_vec(), reference.segments.clone()].concat();
                if self.paths.contains_key(&full) {
                    return Ok(render_id(&full));
                }
                break;
            }
        }
        Err(fail(
            span,
            "document/reference",
            format!(
                "unknown object reference {}",
                serde_json::to_string(&reference.segments).unwrap()
            ),
        )
        .with_optional_help(crate::diagnostic::suggestion(
            first,
            self.paths
                .keys()
                .filter_map(|p| p.last().map(String::as_str)),
        )))
    }
    fn reference_value(
        &self,
        value: &Value,
        scope: &[String],
        span: Span,
    ) -> Result<String, Error> {
        let reference = match value {
            Value::Reference(r) => r.clone(),
            Value::Choice(s) | Value::String(s) if !s.is_empty() => Reference {
                root: false,
                segments: vec![s.clone()],
            },
            _ => {
                return Err(fail(
                    span,
                    "document/reference",
                    "expected an object reference",
                ));
            }
        };
        self.resolve(&reference, scope, span)
    }
    fn kind(
        &self,
        name: &str,
        edge: bool,
        span: Span,
        visiting: &mut BTreeSet<String>,
    ) -> Result<(String, Attributes), Error> {
        let builtins = if edge {
            crate::style::arrow_presets()
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>()
        } else {
            crate::style::presets().keys().cloned().collect()
        };
        if builtins.contains(name) {
            return Ok((name.into(), Attributes::new()));
        }
        let kinds = if edge {
            &self.edge_kinds
        } else {
            &self.node_kinds
        };
        let definition = kinds
            .get(name)
            .ok_or_else(|| fail(span, "document/kind", format!("unknown style `{name}`")))?;
        if !visiting.insert(name.into()) {
            return Err(fail(
                definition.span,
                "document/kind-cycle",
                format!("cyclic style base involving `{name}`"),
            ));
        }
        if visiting.len() > 128 {
            return Err(fail(
                definition.span,
                "document/kind-depth",
                "maximum style inheritance depth is 128",
            ));
        }
        let base = attr_text(&definition.attributes, "base")?
            .unwrap_or_else(|| if edge { "default" } else { "node" }.into());
        let (base, mut attributes) = self.kind(&base, edge, definition.span, visiting)?;
        attributes.extend(
            definition
                .attributes
                .iter()
                .filter(|(key, _)| property(key) != "base")
                .map(|(k, v)| (k.clone(), v.clone())),
        );
        visiting.remove(name);
        Ok((base, attributes))
    }
    fn style_args(&self, base: &str, attrs: &Attributes, edge: bool) -> Result<Vec<Arg>, Error> {
        style_values(attrs, edge)?;
        allowed(
            attrs,
            if edge {
                &["tone", "stroke", "default-label", "legend-label"]
            } else {
                &[
                    "tone",
                    "fill",
                    "font",
                    "align",
                    "role",
                    "shape",
                    "legend-label",
                ]
            },
        )?;
        let mut args = if edge {
            Vec::new()
        } else {
            vec![attr("base", base)]
        };
        if !edge
            && let Some(a) = attrs.get("fill-color")
            && matches!(&a.value, Value::Choice(s) | Value::String(s) if s == "none")
        {
            args.push(Arg::Value(OldValue::Ident("hollow".into())));
        }
        // Fully specify inherited flags, including false values. Legacy flags
        // only express setting a flag, so start from a known built-in base.
        if edge {
            let style = &crate::style::arrow_presets()[base];
            args.push(Arg::Value(OldValue::Ident(
                if style.dashed { "dashed" } else { "solid" }.into(),
            )));
            if let crate::style::ArrowColor::Tone(tone) = style.color {
                args.push(attr("tone", tone.name()));
            }
            if let Some(label) = &style.label {
                args.push(attr("label", label));
            }
        }
        if edge && let Some(style) = super::Paint::parse(attrs, true)?.stroke_style {
            args.push(Arg::Value(OldValue::Ident(
                if style == super::StrokeStyle::Solid {
                    "solid"
                } else {
                    "dashed"
                }
                .into(),
            )));
        }
        for (key, a) in attrs
            .iter()
            .filter(|(key, _)| property(key) == "shape")
            .chain(attrs.iter().filter(|(key, _)| property(key) != "shape"))
        {
            if super::paint::is_paint(key) {
                continue;
            }
            let key = property(key);
            let flag = match key {
                "fill" => Some(match text(&a.value, a.span)?.as_str() {
                    "hollow" => "hollow",
                    "solid" => "filled",
                    _ => {
                        return Err(fail(
                            a.span,
                            "document/value",
                            "fill must be hollow or solid",
                        ));
                    }
                }),
                "font" => Some(match text(&a.value, a.span)?.as_str() {
                    "mono" => "mono",
                    "sans" => "sans",
                    _ => return Err(fail(a.span, "document/value", "font must be mono or sans")),
                }),
                "stroke" => Some(match text(&a.value, a.span)?.as_str() {
                    "dashed" => "dashed",
                    "solid" => "solid",
                    _ => {
                        return Err(fail(
                            a.span,
                            "document/value",
                            "stroke must be dashed or solid",
                        ));
                    }
                }),
                _ => None,
            };
            if let Some(flag) = flag {
                args.push(Arg::Value(OldValue::Ident(flag.into())));
            } else if edge
                && key == "tone"
                && ["target", "source", "auto"].contains(&text(&a.value, a.span)?.as_str())
            {
                args.push(Arg::Value(OldValue::Ident("inherit".into())));
            } else {
                let value = if key == "shape" {
                    let name = text(&a.value, a.span)?;
                    old(&Value::Choice(match name.as_str() { "rectangle" => "rectangle", "rounded-rectangle" => "process", "diamond" => "decision", "capsule" => "terminal", _ => return Err(fail(a.span, "document/shape", "shape must be rectangle, rounded-rectangle, diamond or capsule; select other templates with base=")) }.into()), a.span)?
                } else {
                    old(&a.value, a.span)?
                };
                args.push(Arg::Attr(
                    if key == "legend-label" {
                        "label".into()
                    } else if edge && key == "label" {
                        "chip".into()
                    } else {
                        key.into()
                    },
                    value,
                ));
            }
        }
        Ok(args)
    }
    fn lower(&mut self, body: &[GraphStatement], scope: &[String]) -> Result<Vec<Stmt>, Error> {
        let mut result = Vec::new();
        for statement in body {
            match statement {
                GraphStatement::Node(original) => {
                    let mut node = original.clone();
                    let category = if self.mode == "sequence" {
                        "participant"
                    } else if self.mode == "state-machine" {
                        "state"
                    } else {
                        "node"
                    };
                    let mut attributes = self.defaults.get(category).cloned().unwrap_or_default();
                    attributes.extend(node.attributes.clone());
                    node.attributes = attributes;
                    allowed(
                        &node.attributes,
                        &[
                            "kind",
                            "shape",
                            "tone",
                            "fill",
                            "font",
                            "align",
                            "role",
                            "href",
                            "textdir",
                            "after",
                            "same-layer",
                            "beside",
                        ],
                    )?;
                    style_values(&node.attributes, false)?;
                    if self.mode == "sequence" {
                        for name in ["after", "same-rank", "beside", "shape"] {
                            if let Some(a) = node.attributes.get(name) {
                                return Err(fail(
                                    a.name_span,
                                    "document/participant",
                                    "participants use authored order and their fixed header shape",
                                ));
                            }
                        }
                    }
                    let path = [scope.to_vec(), vec![node.id.clone()]].concat();
                    let id = render_id(&path);
                    let name = attr_text(&node.attributes, "kind")?.unwrap_or_else(|| {
                        if ["participant", "actor"].contains(&node.declaration.as_str()) {
                            "process".into()
                        } else {
                            node.declaration.clone()
                        }
                    });
                    let (mut base, mut style_attrs) =
                        self.kind(&name, false, node.span, &mut BTreeSet::new())?;
                    let has_children = node
                        .body
                        .iter()
                        .any(|s| matches!(s, GraphStatement::Node(_) | GraphStatement::Row { .. }));
                    let shape = crate::style::presets()[&base].shape;
                    let compact_override = attr_text(&style_attrs, "shape")?.is_some_and(|s| {
                        [
                            "process", "decision", "diamond", "terminal", "initial", "final",
                            "choice",
                        ]
                        .contains(&s.as_str())
                    });
                    if has_children
                        && shape != crate::style::Shape::State
                        && (shape.compact() || compact_override)
                    {
                        return Err(fail(
                            node.span,
                            "document/children",
                            "compact nodes cannot contain child diagrams",
                        ));
                    }
                    if has_children && matches!(base.as_str(), "node" | "card") {
                        base = "group".into();
                    }
                    let mut default_style =
                        self.defaults.get(category).cloned().unwrap_or_default();
                    default_style.retain(|key, _| {
                        ["tone", "font", "align", "role", "shape"].contains(&property(key))
                            || super::paint::is_paint(key)
                    });
                    default_style.extend(style_attrs);
                    style_attrs = default_style;
                    for (key, value) in &original.attributes {
                        if ["tone", "fill", "font", "align", "role", "shape"]
                            .contains(&property(key))
                            || super::paint::is_paint(key)
                        {
                            style_attrs.insert(key.clone(), value.clone());
                        }
                    }
                    self.serial += 1;
                    let style = format!("__layup_node_{}", self.serial);
                    let mut style_args = vec![Arg::Value(OldValue::Ident(style.clone()))];
                    style_args.extend(self.style_args(&base, &style_attrs, false)?);
                    self.styles.push(item("style", style_args, None, node.span));
                    let marker = matches!(base.as_str(), "initial" | "final");
                    if marker && (node.label_span.is_some() || !node.body.is_empty()) {
                        return Err(fail(
                            node.span,
                            "document/marker",
                            "initial/final markers take no display label or content",
                        ));
                    }
                    let mut args = vec![attr("id", &id)];
                    if !marker && (node.declaration != "choice" || node.label_span.is_some()) {
                        args.push(attr("title", &node.title));
                    }
                    for (key, a) in &node.attributes {
                        let key = property(key);
                        match key {
                            "href" | "gutter" => {
                                args.push(Arg::Attr(key.into(), old(&a.value, a.span)?))
                            }
                            "textdir" => args
                                .push(Arg::Attr("text-direction".into(), old(&a.value, a.span)?)),
                            "after" | "same-layer" | "beside" => {
                                args.push(attr(key, self.reference_value(&a.value, scope, a.span)?))
                            }
                            _ => {}
                        }
                    }
                    if self.mode != "state-machine" && matches!(base.as_str(), "initial" | "final")
                    {
                        return Err(fail(
                            node.span,
                            "document/kind",
                            "initial/final belong to the state-machine grammar",
                        ));
                    }
                    if self.mode == "state-machine" {
                        let expected = match node.declaration.as_str() {
                            "initial" => crate::style::Shape::Initial,
                            "final" => crate::style::Shape::Final,
                            "choice" => crate::style::Shape::Choice,
                            _ => crate::style::Shape::State,
                        };
                        if crate::style::presets()[&base].shape != expected
                            || style_attrs.contains_key("shape")
                        {
                            return Err(fail(
                                node.span,
                                "document/state-style",
                                "a state style must preserve the declaration's state/initial/final/choice semantics",
                            ));
                        }
                    }
                    if self.mode == "graph"
                        && crate::style::presets()[&base].shape == crate::style::Shape::Choice
                    {
                        return Err(fail(
                            node.span,
                            "document/kind",
                            "choice belongs to the state-machine grammar; use a diamond node in a graph",
                        ));
                    }
                    let mut evidence = annotations(&node.annotations)?;
                    evidence.declaration = Some(node.declaration.clone());
                    evidence.attributes = original.attributes.clone();
                    evidence.paint = super::Paint::parse(&style_attrs, false)?;
                    evidence.path = Some(path.clone());
                    evidence.authored_id = Some(node.id.clone());
                    self.objects.insert(id.clone(), evidence);
                    self.semantic_kinds.insert(id, name);
                    if node
                        .body
                        .iter()
                        .filter(
                            |s| matches!(s, GraphStatement::Content { kind, .. } if kind == "tag"),
                        )
                        .count()
                        > 1
                    {
                        return Err(fail(
                            node.span,
                            "document/content",
                            "a node takes at most one tag",
                        ));
                    }
                    if self.mode == "sequence" {
                        if !matches!(
                            crate::style::presets()[&base].shape,
                            crate::style::Shape::Card
                                | crate::style::Shape::Process
                                | crate::style::Shape::Api
                        ) || style_attrs.contains_key("shape")
                        {
                            return Err(fail(
                                node.span,
                                "document/participant",
                                "participant styles use card/process/interface templates and a fixed header shape",
                            ));
                        }
                        let template = &crate::style::presets()[&base];
                        args.push(Arg::Value(OldValue::Ident(
                            if template.mono { "mono" } else { "sans" }.into(),
                        )));
                        args.push(Arg::Value(OldValue::Ident(
                            if template.hollow { "hollow" } else { "filled" }.into(),
                        )));
                        args.push(attr(
                            "align",
                            match template.align {
                                crate::style::Align::Start => "start",
                                crate::style::Align::End => "end",
                                crate::style::Align::Left => "left",
                                crate::style::Align::Right => "right",
                                crate::style::Align::Center => "center",
                            },
                        ));
                        if let Some(role) = &template.role {
                            args.push(attr("role", role));
                        }
                        if !template.auto {
                            args.push(attr("tone", template.tone.name()));
                        }
                        if !node.body.is_empty() || !scope.is_empty() {
                            return Err(fail(
                                node.span,
                                "document/participant",
                                "participants belong directly to the diagram and have no body",
                            ));
                        }
                        args.extend(self.style_args(&base, &style_attrs, false)?.into_iter().filter(|a| !matches!(a, Arg::Attr(k, _) if k == "base" || k == "shape" || k == "legend-label")));
                        result.push(item(&node.declaration, args, None, node.span));
                    } else {
                        result.push(item(
                            &style,
                            args,
                            Some(self.lower(&node.body, &path)?),
                            node.span,
                        ));
                    }
                }
                GraphStatement::Edge(original) => {
                    let mut edge = original.clone();
                    let category = if self.mode == "sequence" {
                        "message"
                    } else if self.mode == "state-machine" {
                        "transition"
                    } else {
                        "edge"
                    };
                    let mut attributes = self.defaults.get(category).cloned().unwrap_or_default();
                    let style = attr_text(&original.attributes, "kind")?
                        .or(attr_text(&attributes, "kind")?)
                        .unwrap_or_else(|| "default".into());
                    let (_, definition) =
                        self.kind(&style, true, original.span, &mut BTreeSet::new())?;
                    attributes.extend(
                        definition
                            .into_iter()
                            .filter(|(key, _)| *key == "palette" || super::paint::is_paint(key)),
                    );
                    attributes.extend(edge.attributes.clone());
                    edge.attributes = attributes;
                    result.push(self.edge(&edge, scope)?);
                }
                GraphStatement::NodeStyle(_) | GraphStatement::EdgeStyle(_) => {}
                GraphStatement::Row {
                    attributes,
                    body,
                    span,
                    ..
                } => {
                    allowed(attributes, &["weights", "gutter"])?;
                    for cell in body {
                        if let GraphStatement::Layout {
                            kind: LayoutKind::Gap,
                            attributes,
                            span,
                            ..
                        } = cell
                            && attributes.contains_key("size")
                        {
                            return Err(fail(
                                *span,
                                "document/layout",
                                "row gaps are empty cells; an explicit size cannot be honored there",
                            ));
                        }
                    }
                    let mut args = Vec::new();
                    if let Some(a) = attributes.get("weights") {
                        let Value::List(values) = &a.value else {
                            return Err(fail(
                                a.span,
                                "document/value",
                                "row weights require a list",
                            ));
                        };
                        let weights = values
                            .iter()
                            .map(|v| match v {
                                Value::Integer(n) if n.as_f64() > 0.0 => Ok(n.as_f64()),
                                Value::Float(n) if *n > 0.0 => Ok(*n),
                                _ => Err(fail(
                                    a.span,
                                    "document/value",
                                    "row weights must be positive numbers",
                                )),
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        if weights.len()
                            != body
                                .iter()
                                .filter(|s| {
                                    !matches!(
                                        s,
                                        GraphStatement::Edge(_)
                                            | GraphStatement::NodeStyle(_)
                                            | GraphStatement::EdgeStyle(_)
                                    )
                                })
                                .count()
                        {
                            return Err(fail(
                                a.span,
                                "document/value",
                                "row weights must match the number of cells",
                            ));
                        }
                        args.push(Arg::Weights(weights));
                    }
                    if let Some(a) = attributes.get("gap") {
                        args.push(Arg::Attr("gutter".into(), old(&a.value, a.span)?));
                    }
                    result.push(item("row", args, Some(self.lower(body, scope)?), *span));
                }
                GraphStatement::View(block) => result.push(self.named(block, false, scope)?),
                GraphStatement::Step(block) => result.push(self.named(block, true, scope)?),
                GraphStatement::Selection {
                    kind,
                    objects,
                    object_spans,
                    connections,
                    connection_spans,
                    span,
                    ..
                } => {
                    let head = match kind {
                        SelectionKind::Include => "include",
                        SelectionKind::Show => "show",
                        SelectionKind::Highlight => "highlight",
                    };
                    if !objects.is_empty() {
                        let args = objects
                            .iter()
                            .zip(object_spans)
                            .map(|(r, span)| {
                                self.resolve(r, scope, *span)
                                    .map(|id| Arg::Value(OldValue::Ident(id)))
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        let mut statement = item(head, args, None, *span);
                        if let Stmt::Item(i) = &mut statement {
                            i.arg_spans = object_spans.clone();
                        }
                        result.push(statement);
                    }
                    if !connections.is_empty() {
                        let args = connections
                            .iter()
                            .zip(connection_spans)
                            .map(|(id, span)| {
                                if !self.edge_names.contains(id) {
                                    return Err(fail(
                                        *span,
                                        "document/connection",
                                        format!("unknown connection `{id}`"),
                                    ));
                                }
                                Ok(Arg::Value(OldValue::Ident(relationship_id(id))))
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        let mut statement = item(&format!("{head}-edge"), args, None, *span);
                        if let Stmt::Item(i) = &mut statement {
                            i.arg_spans = connection_spans.clone();
                        }
                        result.push(statement);
                    }
                }
                GraphStatement::Fragment {
                    kind,
                    title,
                    body,
                    span,
                    ..
                } => {
                    let head = match kind {
                        FragmentKind::Loop => "loop",
                        FragmentKind::Optional => "opt",
                        FragmentKind::Alternatives => "alt",
                        FragmentKind::Branch => "branch",
                    };
                    result.push(item(
                        head,
                        vec![Arg::Value(OldValue::Str(title.clone()))],
                        Some(self.lower(body, scope)?),
                        *span,
                    ));
                }
                GraphStatement::SequenceNote {
                    text,
                    attributes,
                    span,
                    ..
                } => {
                    allowed(attributes, &["over", "between"])?;
                    if attributes.contains_key("over") && attributes.contains_key("between") {
                        return Err(fail(
                            *span,
                            "document/note",
                            "note accepts either over or between",
                        ));
                    }
                    let mut args = vec![Arg::Value(OldValue::Str(text.clone()))];
                    if let Some(a) = attributes.get("over") {
                        args.push(attr("over", self.reference_value(&a.value, scope, a.span)?));
                    }
                    if let Some(a) = attributes.get("between") {
                        let Value::List(values) = &a.value else {
                            return Err(fail(
                                a.span,
                                "document/note",
                                "between requires two participant references",
                            ));
                        };
                        if values.len() != 2 {
                            return Err(fail(
                                a.span,
                                "document/note",
                                "between requires two participant references",
                            ));
                        }
                        for (name, value) in ["from", "to"].into_iter().zip(values) {
                            args.push(attr(name, self.reference_value(value, scope, a.span)?));
                        }
                    }
                    result.push(item("note", args, None, *span));
                }
                GraphStatement::Layout {
                    kind,
                    title,
                    attributes,
                    body,
                    span,
                    ..
                } => {
                    let head = match kind {
                        LayoutKind::Section => "section",
                        LayoutKind::Band => "band",
                        LayoutKind::Divider => "divider",
                        LayoutKind::Gap => "gap",
                    };
                    allowed(
                        attributes,
                        if *kind == LayoutKind::Gap {
                            &["size"]
                        } else if *kind == LayoutKind::Divider {
                            &["tone"]
                        } else {
                            &[]
                        },
                    )?;
                    if *kind == LayoutKind::Gap && title.is_some() {
                        return Err(fail(*span, "document/layout", "a gap has no display label"));
                    }
                    let mut args = Vec::new();
                    if let Some(title) = title {
                        args.push(Arg::Value(OldValue::Str(title.clone())));
                    }
                    if [LayoutKind::Section, LayoutKind::Band].contains(kind) && title.is_none() {
                        return Err(fail(
                            *span,
                            "document/layout",
                            "section/band require a label",
                        ));
                    }
                    for (key, a) in attributes {
                        if key == "size" {
                            let valid = match &a.value {
                                Value::Integer(n) => n.as_f64() >= 0.,
                                Value::Float(n) => *n >= 0.,
                                _ => false,
                            };
                            if !valid {
                                return Err(fail(
                                    a.value_span,
                                    "document/layout",
                                    "gap size must be a nonnegative number",
                                ));
                            }
                            args.push(Arg::Value(old(&a.value, a.span)?));
                        } else {
                            args.push(Arg::Attr(property(key).into(), old(&a.value, a.span)?));
                        }
                    }
                    result.push(item(
                        head,
                        args,
                        if body.is_empty() {
                            None
                        } else {
                            Some(self.lower(body, scope)?)
                        },
                        *span,
                    ));
                }
                GraphStatement::Configuration {
                    kind: ConfigurationKind::Legend,
                    attributes,
                    span,
                    ..
                } => {
                    allowed(attributes, &["visibility", "position", "nodes", "edges"])?;
                    let visibility =
                        attr_text(attributes, "visibility")?.unwrap_or_else(|| "auto".into());
                    if !["auto", "visible", "hidden"].contains(&visibility.as_str()) {
                        return Err(fail(
                            *span,
                            "document/legend",
                            "visibility must be auto, visible or hidden",
                        ));
                    }
                    let position =
                        attr_text(attributes, "position")?.unwrap_or_else(|| "top".into());
                    if !["top", "bottom"].contains(&position.as_str()) {
                        return Err(fail(
                            *span,
                            "document/legend",
                            "position must be top or bottom",
                        ));
                    }
                    let mut args = vec![Arg::Value(OldValue::Ident(if visibility == "hidden" {
                        "off".into()
                    } else {
                        position
                    }))];
                    if visibility == "auto" {
                        args.push(Arg::Value(OldValue::Ident("auto".into())));
                    }
                    for key in ["nodes", "edges"] {
                        if let Some(a) = attributes.get(key) {
                            let Value::List(values) = &a.value else {
                                return Err(fail(
                                    a.span,
                                    "document/legend",
                                    "style filters require a list",
                                ));
                            };
                            let names = values
                                .iter()
                                .map(|v| text(v, a.span))
                                .collect::<Result<Vec<_>, _>>()?;
                            for name in &names {
                                self.kind(name, key == "edges", a.span, &mut BTreeSet::new())?;
                            }
                            args.push(attr(
                                if key == "edges" { "arrows" } else { "nodes" },
                                names.join(","),
                            ));
                        }
                    }
                    result.push(item("legend", args, None, *span));
                }
                GraphStatement::Configuration {
                    kind: ConfigurationKind::Slide,
                    ..
                }
                | GraphStatement::Defaults { .. } => {}
                GraphStatement::Port { span, .. } => {
                    return Err(fail(
                        *span,
                        "document/port",
                        "named member ports are not implemented yet; use source-side and target-side",
                    ));
                }
                GraphStatement::Content {
                    kind, text, span, ..
                } => {
                    if scope.is_empty() && !["text", "speaker-note"].contains(&kind.as_str()) {
                        return Err(fail(
                            *span,
                            "document/content",
                            "code and tag content require a containing node",
                        ));
                    }
                    result.push(item(
                        if kind == "speaker-note" {
                            "note"
                        } else if kind == "entry" || kind == "exit" {
                            "code"
                        } else {
                            kind
                        },
                        vec![Arg::Value(OldValue::Str(
                            if kind == "entry" || kind == "exit" {
                                format!("{kind} / {text}")
                            } else {
                                text.clone()
                            },
                        ))],
                        None,
                        *span,
                    ));
                }
            }
        }
        Ok(result)
    }
    fn named(&mut self, block: &NamedBlock, step: bool, scope: &[String]) -> Result<Stmt, Error> {
        allowed(
            &block.attributes,
            if step {
                &[]
            } else {
                &["direction", "width", "layout", "textdir"]
            },
        )?;
        if !step {
            direction_options(&block.attributes, &self.mode)?;
        }
        let mut args = vec![
            Arg::Value(OldValue::Ident(block.id.clone())),
            Arg::Value(OldValue::Str(block.title.clone())),
        ];
        for (key, a) in &block.attributes {
            args.push(Arg::Attr(
                if property(key) == "textdir" {
                    "text-direction".into()
                } else {
                    property(key).into()
                },
                old(&a.value, a.span)?,
            ));
        }
        let mut body = Vec::new();
        let mut has_slide = false;
        for statement in &block.body {
            if let GraphStatement::Configuration {
                kind: ConfigurationKind::Slide,
                attributes,
                span,
                ..
            } = statement
            {
                if step || has_slide {
                    return Err(fail(
                        *span,
                        "document/slide",
                        "a view takes at most one slide configuration",
                    ));
                }
                has_slide = true;
                args.extend(slide_args(attributes, *span)?);
            } else {
                body.push(statement.clone());
            }
        }
        let mut statement = item(
            if step { "step" } else { "view" },
            args,
            Some(self.lower(&body, scope)?),
            block.span,
        );
        if let Stmt::Item(i) = &mut statement {
            i.arg_spans[0] = block.id_span;
        }
        Ok(statement)
    }
    fn edge(&mut self, edge: &Edge, scope: &[String]) -> Result<Stmt, Error> {
        allowed(
            &edge.attributes,
            &[
                "kind",
                "tone",
                "stroke",
                "source-side",
                "target-side",
                "via",
                "bus",
                "label",
                "type",
                "delivery",
                "event",
                "guard",
                "action",
            ],
        )?;
        style_values(&edge.attributes, true)?;
        let mut from = self.resolve(&edge.from, scope, edge.from_span)?;
        let mut to = self.resolve(&edge.to, scope, edge.to_span)?;
        if edge.arrow == "<-" {
            std::mem::swap(&mut from, &mut to);
        }
        let kind = attr_text(&edge.attributes, "kind")?.unwrap_or_else(|| "default".into());
        let (base, attrs) = self.kind(&kind, true, edge.span, &mut BTreeSet::new())?;
        let mut paint_attrs = attrs.clone();
        paint_attrs.extend(
            edge.attributes
                .iter()
                .filter(|(key, _)| super::paint::is_paint(key) || *key == "palette")
                .map(|(k, v)| (k.clone(), v.clone())),
        );
        let mut style_args = vec![Arg::Value(OldValue::Ident(kind.clone()))];
        style_args.extend(self.style_args(&base, &attrs, true)?);
        self.styles.push(item("arrow", style_args, None, edge.span));
        let id = match &edge.id {
            Some(id) => relationship_id(id),
            None => format!("anonymous:{}", self.relationships.len()),
        };
        let mut args = vec![attr("id", &id)];
        if let Some(label) = &edge.label {
            args.push(attr("label", label));
        }
        for (key, a) in &edge.attributes {
            let key = property(key);
            match key {
                "source-side" | "target-side" | "via" | "tone"
                    if key != "tone"
                        || !["source", "target", "auto"]
                            .contains(&text(&a.value, a.span)?.as_str()) =>
                {
                    args.push(Arg::Attr(
                        match key {
                            "source-side" => "from",
                            "target-side" => "to",
                            other => other,
                        }
                        .into(),
                        old(&a.value, a.span)?,
                    ))
                }
                "stroke" => match text(&a.value, a.span)?.as_str() {
                    "dashed" | "dotted" => args.push(Arg::Value(OldValue::Ident("dashed".into()))),
                    "solid" => args.push(Arg::Value(OldValue::Ident("solid".into()))),
                    _ => {
                        return Err(fail(
                            a.span,
                            "document/value",
                            "stroke must be solid or dashed",
                        ));
                    }
                },
                _ => {}
            }
        }
        let mut evidence = annotations(&edge.annotations)?;
        evidence.declaration = Some(
            if self.mode == "sequence" {
                "message"
            } else if self.mode == "state-machine" {
                "transition"
            } else {
                "edge"
            }
            .into(),
        );
        evidence.attributes = edge.attributes.clone();
        evidence.paint = super::Paint::parse(&paint_attrs, true)?;
        evidence.authored_id = edge.id.clone();
        self.relationships.insert(id, evidence);
        if let Some(a) = edge.attributes.get("bus") {
            match a.value {
                Value::Bool(true) => args.push(Arg::Value(OldValue::Ident("bus".into()))),
                Value::Bool(false) => {}
                _ => return Err(fail(a.span, "document/value", "bus requires true or false")),
            }
        }
        if let Some(a) = edge.attributes.get("label") {
            if edge.label.is_some() || text(&a.value, a.span)? != "style" {
                return Err(fail(
                    a.span,
                    "document/value",
                    "label=style is exclusive with a positional label",
                ));
            }
            let caption = attr_text(&attrs, "label")?
                .or_else(|| {
                    crate::style::arrow_presets()[&base]
                        .chip
                        .clone()
                        .or_else(|| crate::style::arrow_presets()[&base].label.clone())
                })
                .ok_or_else(|| {
                    fail(
                        a.value_span,
                        "document/label",
                        "label=style requires a default-label on the selected edge style",
                    )
                })?;
            args.push(attr("label", caption));
        }
        if self.mode == "sequence" {
            for name in [
                "source-side",
                "target-side",
                "route-side",
                "bus",
                "event",
                "guard",
                "action",
            ] {
                if let Some(a) = edge.attributes.get(name) {
                    return Err(fail(
                        a.span,
                        "document/message",
                        "sequence messages cannot use graph routing or transition properties",
                    ));
                }
            }
            let message_type =
                attr_text(&edge.attributes, "type")?.unwrap_or_else(|| "call".into());
            let delivery = attr_text(&edge.attributes, "delivery")?;
            if !["call", "reply"].contains(&message_type.as_str())
                || (message_type == "reply" && delivery.is_some())
            {
                return Err(fail(
                    edge.span,
                    "document/message",
                    "type must be call or reply; delivery applies only to calls",
                ));
            }
            if message_type == "reply" {
                args.push(Arg::Value(OldValue::Ident("return".into())));
            }
            if let Some(delivery) = delivery {
                if !["sync", "async"].contains(&delivery.as_str()) {
                    return Err(fail(
                        edge.span,
                        "document/message",
                        "delivery must be sync or async",
                    ));
                }
                if delivery == "async" {
                    args.push(Arg::Value(OldValue::Ident("async".into())));
                }
            }
        } else if edge.attributes.contains_key("type") || edge.attributes.contains_key("delivery") {
            return Err(fail(
                edge.span,
                "document/edge",
                "type and delivery apply to sequence messages",
            ));
        }
        let transition = ["event", "guard", "action"]
            .iter()
            .any(|key| edge.attributes.contains_key(*key));
        if transition {
            if self.mode != "state-machine" || edge.label.is_some() {
                return Err(fail(
                    edge.span,
                    "document/transition",
                    "event/guard/action require a transition without a positional label",
                ));
            }
            let event = attr_text(&edge.attributes, "event")?.unwrap_or_default();
            let guard =
                attr_text(&edge.attributes, "guard")?.map_or(String::new(), |g| format!(" [{g}]"));
            let action =
                attr_text(&edge.attributes, "action")?.map_or(String::new(), |a| format!(" / {a}"));
            args.push(attr("label", format!("{event}{guard}{action}").trim()));
        }
        let arg_spans = vec![edge.span; args.len()];
        Ok(Stmt::Edge(EdgeStmt {
            from,
            to,
            kind: Some(kind),
            left: edge.arrow == "<->",
            right: edge.arrow != "--",
            args,
            line: edge.span.line,
            span: edge.span,
            from_span: edge.from_span,
            to_span: edge.to_span,
            arrow_span: edge.arrow_span,
            arg_spans,
        }))
    }
}

struct Lowered {
    statements: Vec<Stmt>,
    diagram: crate::model::Diagram,
    info: Info,
    semantic_kinds: BTreeMap<String, String>,
}

fn lower(
    document: &Document,
    options: &CompileOptions,
    fonts: Option<&Fonts>,
) -> Result<Lowered, Error> {
    let mut outputs = Vec::new();
    for diagram in &document.diagrams {
        let Some(body) = diagram.body.statements() else {
            annotations(&diagram.annotations)?;
            continue;
        };
        allowed(
            &diagram.attributes,
            &[
                "type",
                "background-color",
                "width",
                "layout",
                "direction",
                "textdir",
                "subtitle",
            ],
        )?;
        direction_options(&diagram.attributes, &diagram.diagram_type)?;
        let mut builder = Builder {
            paths: BTreeMap::new(),
            node_kinds: BTreeMap::new(),
            edge_kinds: BTreeMap::new(),
            styles: Vec::new(),
            objects: BTreeMap::new(),
            relationships: BTreeMap::new(),
            edge_names: BTreeSet::new(),
            semantic_kinds: BTreeMap::new(),
            serial: 0,
            mode: diagram.diagram_type.clone(),
            defaults: BTreeMap::new(),
        };
        builder.collect(body, &[], true)?;
        builder.validate_shadowing()?;
        for (category, attrs) in &builder.defaults {
            allowed(
                attrs,
                if ["edge", "message", "transition"].contains(&category.as_str()) {
                    &[
                        "kind",
                        "tone",
                        "stroke",
                        "bus",
                        "label",
                        "source-side",
                        "target-side",
                        "via",
                        "type",
                        "delivery",
                        "event",
                        "guard",
                        "action",
                    ]
                } else {
                    &[
                        "kind",
                        "tone",
                        "font",
                        "align",
                        "role",
                        "href",
                        "textdir",
                        "after",
                        "same-layer",
                        "beside",
                    ]
                },
            )?;
            let edge = ["edge", "message", "transition"].contains(&category.as_str());
            style_values(attrs, edge)?;
            super::Paint::parse(attrs, edge)?;
            let invalid: &[&str] = if diagram.diagram_type == "sequence" {
                &[
                    "after",
                    "same-rank",
                    "beside",
                    "shape",
                    "source-side",
                    "target-side",
                    "route-side",
                    "bus",
                    "event",
                    "guard",
                    "action",
                ]
            } else if diagram.diagram_type == "graph" {
                &["type", "delivery", "event", "guard", "action"]
            } else {
                &["type", "delivery"]
            };
            for name in invalid {
                if let Some(a) = attrs.get(*name) {
                    return Err(fail(
                        a.name_span,
                        "document/defaults",
                        "this default has no semantics in the selected diagram grammar",
                    ));
                }
            }
        }
        // Validate unused kind declarations too, before selection/layout.
        let mut validated_styles = Vec::new();
        for (edge, kinds) in [(false, &builder.node_kinds), (true, &builder.edge_kinds)] {
            for (name, kind) in kinds {
                allowed(
                    &kind.attributes,
                    if edge {
                        &["base", "tone", "stroke", "default-label", "legend-label"]
                    } else {
                        &[
                            "base",
                            "tone",
                            "fill",
                            "font",
                            "align",
                            "role",
                            "shape",
                            "legend-label",
                        ]
                    },
                )?;
                let (base, attrs) = builder.kind(name, edge, kind.span, &mut BTreeSet::new())?;
                super::Paint::parse(&attrs, edge)?;
                let mut args = vec![Arg::Value(OldValue::Ident(if edge {
                    name.clone()
                } else {
                    format!("__layup_kind_{name}")
                }))];
                args.extend(builder.style_args(&base, &attrs, edge)?);
                validated_styles.push(item(
                    if edge { "arrow" } else { "style" },
                    args,
                    None,
                    kind.span,
                ));
            }
        }
        builder.styles.extend(validated_styles);
        for name in [
            "fill-color",
            "stroke-color",
            "text-color",
            "stroke-style",
            "stroke-width",
        ] {
            if let Some(a) = diagram.attributes.get(name) {
                return Err(fail(
                    a.span,
                    "document/attribute",
                    "diagram paint accepts background-color",
                ));
            }
        }
        let typed_body = body;
        let mut body = builder.lower(body, &[])?;
        let mut args = vec![attr("title", &diagram.title)];
        if diagram.diagram_type != "graph" {
            args.push(attr("mode", &diagram.diagram_type));
        }
        let mut configurations = BTreeSet::new();
        for statement in typed_body {
            if let GraphStatement::Configuration {
                kind,
                attributes,
                span,
                ..
            } = statement
            {
                let name = if *kind == ConfigurationKind::Slide {
                    "slide"
                } else {
                    "legend"
                };
                if !configurations.insert(name) {
                    return Err(fail(
                        *span,
                        "document/configuration",
                        format!("duplicate {name} configuration"),
                    ));
                }
                if *kind == ConfigurationKind::Slide {
                    args.extend(slide_args(attributes, *span)?);
                }
            }
        }
        for (key, a) in &diagram.attributes {
            let key = property(key);
            if key == "kind" || key == "type" || key == "subtitle" || super::paint::is_paint(key) {
                continue;
            }
            args.push(Arg::Attr(
                if key == "textdir" {
                    "text-direction".into()
                } else {
                    key.into()
                },
                old(&a.value, a.span)?,
            ));
        }
        let mut evidence = annotations(&diagram.annotations)?;
        evidence.declaration = Some("diagram".into());
        evidence.attributes = diagram.attributes.clone();
        evidence.paint = super::Paint::diagram(&diagram.attributes)?;
        if let Some(doc) = &evidence.documentation {
            body.insert(
                0,
                item(
                    "desc",
                    vec![Arg::Value(OldValue::Str(doc.clone()))],
                    None,
                    diagram.span,
                ),
            );
        }
        if let Some(subtitle) = attr_text(&diagram.attributes, "subtitle")? {
            body.insert(
                0,
                item(
                    if diagram.diagram_type == "sequence" {
                        "subtitle"
                    } else {
                        "note"
                    },
                    vec![Arg::Value(OldValue::Str(subtitle))],
                    None,
                    diagram.span,
                ),
            );
        }
        builder.styles.extend(body);
        let has_views = typed_body
            .iter()
            .any(|s| matches!(s, GraphStatement::View(_)));
        let statements = vec![item(
            if has_views { "model" } else { "diagram" },
            args,
            Some(std::mem::take(&mut builder.styles)),
            diagram.span,
        )];
        // Validate every diagram before choosing one, including unused styles.
        let mut full = statements.clone();
        if let Stmt::Item(root) = &mut full[0] {
            root.head = "diagram".into();
            root.body
                .as_mut()
                .unwrap()
                .retain(|s| !matches!(s, Stmt::Item(i) if i.head == "view"));
        }
        crate::presentation::extract(&mut full)?;
        if let Stmt::Item(root) = &mut full[0] {
            crate::slides::Slide::take(root)?;
        }
        let mut model = if diagram.diagram_type == "sequence" {
            crate::sequence::build(&full)?.diagram
        } else {
            crate::model::build_statements_with_tags(&full, false)?
        };
        for (edge, kinds) in [(false, &builder.node_kinds), (true, &builder.edge_kinds)] {
            for (name, kind) in kinds {
                let (_, attributes) = builder.kind(name, edge, kind.span, &mut BTreeSet::new())?;
                let paint = super::Paint::parse(&attributes, edge)?;
                if edge {
                    model.arrows.get_mut(name).expect("validated style").paint = paint;
                } else {
                    model
                        .kinds
                        .get_mut(&format!("__layup_kind_{name}"))
                        .expect("validated style")
                        .paint = paint;
                }
            }
        }
        // Resolve every view and parse every step before selecting output.
        if has_views {
            let resolved = crate::views::resolve(&statements, None)?;
            for view in resolved.views {
                let mut v = crate::views::resolve(&statements, Some(&view.id))?;
                crate::presentation::extract(&mut v.statements)?;
                if let Some(fonts) = fonts {
                    crate::compile_lowered_with_semantics(
                        &statements,
                        &CompileOptions {
                            view: Some(view.id),
                            diagram: None,
                        },
                        fonts,
                        false,
                        Some((&builder.semantic_kinds, &model)),
                    )?;
                }
            }
        }
        let kinds = builder
            .node_kinds
            .keys()
            .map(|name| {
                (
                    name.clone(),
                    model.kinds[&format!("__layup_kind_{name}")].clone(),
                )
            })
            .collect::<Vec<_>>();
        model.kinds.retain(|name, _| {
            !name.starts_with("__layup_node_") && !name.starts_with("__layup_kind_")
        });
        model.kinds.extend(kinds);
        outputs.push(Lowered {
            statements,
            diagram: model,
            info: Info {
                language_version: document.version,
                diagram_id: diagram.id.clone(),
                diagrams: document.diagrams.iter().map(|d| d.id.clone()).collect(),
                manifest: document
                    .diagrams
                    .iter()
                    .map(|d| ManifestEntry {
                        id: d.id.clone(),
                        diagram_type: d.diagram_type.clone(),
                        status: if d.id == diagram.id {
                            "selected"
                        } else if matches!(d.body, Body::Opaque { .. }) {
                            "skipped"
                        } else {
                            "supported"
                        }
                        .into(),
                        reason: matches!(d.body, Body::Opaque { .. })
                            .then(|| "body grammar unavailable".into()),
                        annotations: d.annotations.clone(),
                        span: d.span,
                    })
                    .collect(),
                targets: targets(document)?,
                diagnostics: document
                    .warnings
                    .iter()
                    .map(|d| serde_json::from_str(&d.json()).expect("diagnostic JSON"))
                    .collect(),
                diagram: evidence,
                objects: builder.objects,
                relationships: builder.relationships,
            },
            semantic_kinds: builder.semantic_kinds,
        });
    }
    let selected = match &options.diagram {
        Some(id) => {
            let diagram = document
                .diagrams
                .iter()
                .find(|d| &d.id == id)
                .ok_or_else(|| {
                    fail(
                        document.diagrams[0].span,
                        "document/diagram",
                        format!("unknown diagram `{id}`"),
                    )
                })?;
            if matches!(diagram.body, Body::Opaque { .. }) {
                return Err(fail(
                    diagram.type_span,
                    "diagram/unavailable-selection",
                    format!(
                        "cannot render diagram `{id}`: type `{}` is unavailable",
                        diagram.diagram_type
                    ),
                ));
            }
            id.as_str()
        }
        None => outputs
            .first()
            .map(|output| output.info.diagram_id.as_str())
            .ok_or_else(|| {
                fail(
                    document.diagrams[0].span,
                    "diagram/no-renderable-diagram",
                    "document has no supported diagram to render; use the inspect or format API",
                )
            })?,
    };
    let index = outputs
        .iter()
        .position(|output| output.info.diagram_id == selected)
        .expect("validated selected diagram");
    Ok(outputs.swap_remove(index))
}

pub(super) fn build(document: &Document) -> Result<crate::model::Diagram, Error> {
    let mut lowered = lower(document, &CompileOptions::default(), None)?;
    let mut resolved = crate::views::resolve(&lowered.statements, None)?;
    crate::presentation::extract(&mut resolved.statements)?;
    if let [Stmt::Item(root)] = resolved.statements.as_mut_slice() {
        crate::slides::Slide::take(root)?;
    }
    let kinds = lowered.diagram.kinds;
    lowered.diagram = if crate::sequence::is_sequence(&resolved.statements) {
        crate::sequence::build(&resolved.statements)?.diagram
    } else {
        crate::model::build_statements_with_tags(&resolved.statements, false)?
    };
    lowered.diagram.kinds = kinds;
    restore_kinds(
        &mut lowered.diagram.blocks,
        &lowered.semantic_kinds,
        Some(&lowered.info.objects),
    );
    Ok(lowered.diagram)
}

pub(crate) fn restore_styles(
    diagram: &mut crate::model::Diagram,
    names: &BTreeMap<String, String>,
) {
    let definitions = diagram
        .kinds
        .iter()
        .filter_map(|(name, style)| {
            name.strip_prefix("__layup_kind_")
                .map(|name| (name.to_owned(), style.clone()))
        })
        .collect::<Vec<_>>();
    diagram
        .kinds
        .retain(|name, _| !name.starts_with("__layup_node_") && !name.starts_with("__layup_kind_"));
    diagram.kinds.extend(definitions);
    restore_kinds(&mut diagram.blocks, names, None);
}

fn restore_kinds(
    blocks: &mut [crate::model::Block],
    kinds: &BTreeMap<String, String>,
    entities: Option<&BTreeMap<String, EntityInfo>>,
) {
    use crate::model::Block;
    for block in blocks {
        match block {
            Block::Node(node) => {
                if let Some(kind) = kinds.get(&node.id) {
                    node.kind = kind.clone();
                }
                if let Some(entity) = entities.and_then(|values| values.get(&node.id)) {
                    node.style.paint = entity.paint.clone();
                }
                restore_kinds(&mut node.children, kinds, entities);
            }
            Block::Row(row) => {
                for cell in row.cells.iter_mut().flatten() {
                    restore_kinds(std::slice::from_mut(cell), kinds, entities);
                }
            }
            Block::Section(section) => restore_kinds(&mut section.children, kinds, entities),
            Block::Tree { nodes, .. } => restore_kinds(nodes, kinds, entities),
            Block::Flow { layers, .. } => {
                for layer in layers {
                    restore_kinds(layer, kinds, entities);
                }
            }
            _ => {}
        }
    }
}

fn recolor_caption(item: &mut crate::layout::Item, palette: crate::style::Tone) {
    use crate::layout::{Ink, Item};
    match item {
        Item::Group(items) => {
            for item in items {
                recolor_caption(item, palette);
            }
        }
        Item::Chip { tone, .. } | Item::Box { tone, .. } => *tone = palette,
        Item::Text(text) if matches!(text.ink, Ink::Tone(_)) => text.ink = Ink::Tone(palette),
        _ => {}
    }
}
pub(super) fn compile(
    document: &Document,
    options: &CompileOptions,
    fonts: &Fonts,
) -> Result<Compiled, Error> {
    let Lowered {
        statements,
        diagram,
        info,
        semantic_kinds,
    } = lower(document, options, Some(fonts))?;
    let mut compiled = crate::compile_lowered_with_semantics(
        &statements,
        &CompileOptions {
            view: options.view.clone(),
            diagram: None,
        },
        fonts,
        false,
        Some((&semantic_kinds, &diagram)),
    )?;
    restore_kinds(
        &mut compiled.diagram.blocks,
        &semantic_kinds,
        Some(&info.objects),
    );
    compiled.diagram.kinds = diagram.kinds;
    for node in &mut compiled.scene.nodes {
        if let Some(kind) = semantic_kinds.get(&node.id) {
            node.kind = kind.clone();
        }
    }
    compiled
        .warnings
        .extend(document.warnings.iter().map(|d| crate::Warning {
            line: d.line,
            msg: format!(
                "[{}] {}{}",
                d.code,
                d.message,
                d.help
                    .as_ref()
                    .map_or(String::new(), |help| format!("; {help}"))
            ),
        }));
    for edge in &mut compiled.scene.edges {
        if let Some(paint) = info.relationships.get(&edge.id).map(|entity| &entity.paint) {
            if let Some(style) = paint.stroke_style {
                edge.dashed = style != super::StrokeStyle::Solid;
            }
            if let Some(origin) = paint.palette_origin {
                let target = if origin == super::PaletteOrigin::Source {
                    &edge.from
                } else {
                    &edge.to
                };
                if let Some(node) = compiled.scene.nodes.iter().find(|n| &n.id == target) {
                    edge.tone = node.tone;
                    if let Some(chip) = &mut edge.chip {
                        recolor_caption(chip, node.tone);
                    }
                }
            }
        }
    }
    compiled.document = Some(info);
    Ok(compiled)
}
