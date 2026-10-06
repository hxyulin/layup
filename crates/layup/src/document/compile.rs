use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::{Value as Json, json};

use super::{Annotation, Attributes, Body, Document, Edge, GraphStatement, Kind, Reference, Value};
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
    pub diagram: EntityInfo,
    pub objects: BTreeMap<String, EntityInfo>,
    pub relationships: BTreeMap<String, EntityInfo>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityInfo {
    pub path: Option<Vec<String>>,
    pub authored_id: Option<String>,
    pub documentation: Option<String>,
    pub source_locations: Vec<SourceLocation>,
    pub metadata: BTreeMap<String, Json>,
}

fn fail(span: Span, code: &'static str, message: impl Into<String>) -> Error {
    Error::located(span, code, message)
}
fn text(value: &Value, span: Span) -> Result<String, Error> {
    match value {
        Value::Word(s) | Value::String(s) => Ok(s.clone()),
        _ => Err(fail(span, "document/value", "expected a word or string")),
    }
}
fn old(value: &Value, span: Span) -> Result<OldValue, Error> {
    match value {
        Value::Word(s) => Ok(OldValue::Ident(s.clone())),
        Value::String(s) => Ok(OldValue::Str(s.clone())),
        Value::Number(n) => Ok(OldValue::Num(*n)),
        _ => Err(fail(
            span,
            "document/value",
            "this property requires a scalar word, string or number",
        )),
    }
}
fn allowed(attrs: &Attributes, names: &[&str]) -> Result<(), Error> {
    for (name, a) in attrs {
        if !names.contains(&name.as_str()) {
            return Err(fail(
                a.span,
                "document/attribute",
                format!("unsupported attribute `{name}` for this declaration"),
            ));
        }
    }
    Ok(())
}
fn attr_text(attrs: &Attributes, name: &str) -> Result<Option<String>, Error> {
    attrs.get(name).map(|a| text(&a.value, a.span)).transpose()
}
fn json_value(value: &Value, span: Span) -> Result<Json, Error> {
    Ok(match value {
        Value::Word(s) | Value::String(s) => json!(s),
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 && *n < u64::MAX as f64 => {
            json!(*n as u64)
        }
        Value::Number(n) => json!(n),
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
    let mut info = EntityInfo::default();
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
                        let range: crate::input::SourceRange =
                            serde_json::from_value(json_value(&a.value, a.span)?).map_err(|e| {
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
            _ => {
                return Err(fail(
                    annotation.span,
                    "document/annotation",
                    format!("unknown annotation @{}", annotation.name),
                ));
            }
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
                GraphStatement::Row { body, .. } => self.collect(body, scope, false)?,
                GraphStatement::NodeKind(kind) | GraphStatement::EdgeKind(kind) => {
                    if !diagram_level {
                        return Err(fail(
                            kind.span,
                            "document/kind",
                            "kind declarations belong directly to the diagram",
                        ));
                    }
                    let kinds = if matches!(statement, GraphStatement::NodeKind(_)) {
                        &mut self.node_kinds
                    } else {
                        &mut self.edge_kinds
                    };
                    let builtin = if matches!(statement, GraphStatement::NodeKind(_)) {
                        crate::style::presets().contains_key(&kind.id)
                    } else {
                        crate::style::arrow_presets().contains_key(&kind.id)
                    };
                    if builtin || kinds.insert(kind.id.clone(), kind).is_some() {
                        return Err(fail(
                            kind.span,
                            "document/kind",
                            format!("duplicate or built-in kind `{}`", kind.id),
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
        ))
    }
    fn reference_value(
        &self,
        value: &Value,
        scope: &[String],
        span: Span,
    ) -> Result<String, Error> {
        let reference = match value {
            Value::Reference(r) => r.clone(),
            Value::Word(s) | Value::String(s) if !s.is_empty() => Reference {
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
            .ok_or_else(|| fail(span, "document/kind", format!("unknown kind `{name}`")))?;
        if !visiting.insert(name.into()) {
            return Err(fail(
                definition.span,
                "document/kind-cycle",
                format!("cyclic kind base involving `{name}`"),
            ));
        }
        if visiting.len() > 128 {
            return Err(fail(
                definition.span,
                "document/kind-depth",
                "maximum kind inheritance depth is 128",
            ));
        }
        let base = attr_text(&definition.attributes, "base")?
            .unwrap_or_else(|| if edge { "default" } else { "node" }.into());
        let (base, mut attributes) = self.kind(&base, edge, definition.span, visiting)?;
        attributes.extend(
            definition
                .attributes
                .iter()
                .filter(|(key, _)| key.as_str() != "base")
                .map(|(k, v)| (k.clone(), v.clone())),
        );
        visiting.remove(name);
        Ok((base, attributes))
    }
    fn style_args(&self, base: &str, attrs: &Attributes, edge: bool) -> Result<Vec<Arg>, Error> {
        allowed(
            attrs,
            if edge {
                &["tone", "stroke", "label"]
            } else {
                &["tone", "fill", "font", "align", "role", "shape"]
            },
        )?;
        let mut args = if edge {
            Vec::new()
        } else {
            vec![attr("base", base)]
        };
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
        for (key, a) in attrs
            .iter()
            .filter(|(key, _)| key.as_str() == "shape")
            .chain(attrs.iter().filter(|(key, _)| key.as_str() != "shape"))
        {
            let flag = match key.as_str() {
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
            } else {
                args.push(Arg::Attr(key.clone(), old(&a.value, a.span)?));
            }
        }
        Ok(args)
    }
    fn lower(&mut self, body: &[GraphStatement], scope: &[String]) -> Result<Vec<Stmt>, Error> {
        let mut result = Vec::new();
        for statement in body {
            match statement {
                GraphStatement::Node(node) => {
                    allowed(
                        &node.attributes,
                        &[
                            "kind",
                            "tone",
                            "fill",
                            "font",
                            "align",
                            "role",
                            "href",
                            "textdir",
                            "gutter",
                            "after",
                            "same-layer",
                            "beside",
                        ],
                    )?;
                    let path = [scope.to_vec(), vec![node.id.clone()]].concat();
                    let id = render_id(&path);
                    let name = attr_text(&node.attributes, "kind")?
                        .unwrap_or_else(|| node.declaration.clone());
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
                    if has_children && (shape.compact() || compact_override) {
                        return Err(fail(
                            node.span,
                            "document/children",
                            "compact nodes cannot contain child diagrams",
                        ));
                    }
                    if has_children && matches!(base.as_str(), "node" | "card") {
                        base = "group".into();
                    }
                    for (key, value) in &node.attributes {
                        if ["tone", "fill", "font", "align", "role"].contains(&key.as_str()) {
                            style_attrs.insert(key.clone(), value.clone());
                        }
                    }
                    self.serial += 1;
                    let style = format!("__layup_node_{}", self.serial);
                    let mut style_args = vec![Arg::Value(OldValue::Ident(style.clone()))];
                    style_args.extend(self.style_args(&base, &style_attrs, false)?);
                    self.styles.push(item("style", style_args, None, node.span));
                    let mut args = vec![attr("id", &id), attr("title", &node.title)];
                    for (key, a) in &node.attributes {
                        match key.as_str() {
                            "href" | "gutter" => {
                                args.push(Arg::Attr(key.clone(), old(&a.value, a.span)?))
                            }
                            "textdir" => args
                                .push(Arg::Attr("text-direction".into(), old(&a.value, a.span)?)),
                            "after" | "same-layer" | "beside" => {
                                args.push(attr(key, self.reference_value(&a.value, scope, a.span)?))
                            }
                            _ => {}
                        }
                    }
                    if matches!(base.as_str(), "initial" | "final") {
                        return Err(fail(
                            node.span,
                            "document/kind",
                            "initial/final belong to the state-machine grammar",
                        ));
                    }
                    let mut evidence = annotations(&node.annotations)?;
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
                    result.push(item(
                        &style,
                        args,
                        Some(self.lower(&node.body, &path)?),
                        node.span,
                    ));
                }
                GraphStatement::Edge(edge) => result.push(self.edge(edge, scope)?),
                GraphStatement::NodeKind(_) | GraphStatement::EdgeKind(_) => {}
                GraphStatement::Row {
                    attributes,
                    body,
                    span,
                } => {
                    allowed(attributes, &["weights", "gutter"])?;
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
                                Value::Number(n) if *n > 0.0 => Ok(*n),
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
                                            | GraphStatement::NodeKind(_)
                                            | GraphStatement::EdgeKind(_)
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
                    if let Some(a) = attributes.get("gutter") {
                        args.push(Arg::Attr("gutter".into(), old(&a.value, a.span)?));
                    }
                    result.push(item("row", args, Some(self.lower(body, scope)?), *span));
                }
                GraphStatement::Content { kind, text, span } => {
                    if scope.is_empty() && kind != "text" {
                        return Err(fail(
                            *span,
                            "document/content",
                            "code and tag content require a containing node",
                        ));
                    }
                    result.push(item(
                        kind,
                        vec![Arg::Value(OldValue::Str(text.clone()))],
                        None,
                        *span,
                    ));
                }
            }
        }
        Ok(result)
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
            ],
        )?;
        let mut from = self.resolve(&edge.from, scope, edge.span)?;
        let mut to = self.resolve(&edge.to, scope, edge.span)?;
        if edge.arrow == "<-" {
            std::mem::swap(&mut from, &mut to);
        }
        let kind = attr_text(&edge.attributes, "kind")?.unwrap_or_else(|| "default".into());
        let (base, attrs) = self.kind(&kind, true, edge.span, &mut BTreeSet::new())?;
        let mut style_args = vec![Arg::Value(OldValue::Ident(kind.clone()))];
        style_args.extend(self.style_args(&base, &attrs, true)?);
        self.styles.push(item("arrow", style_args, None, edge.span));
        let id = match &edge.id {
            Some(id) => {
                if !self.edge_names.insert(id.clone()) {
                    return Err(fail(
                        edge.span,
                        "document/duplicate-edge",
                        format!("duplicate relationship `{id}`"),
                    ));
                }
                format!("relationship:{}", serde_json::to_string(id).unwrap())
            }
            None => format!("anonymous:{}", self.relationships.len()),
        };
        let mut args = vec![attr("id", &id)];
        if let Some(label) = &edge.label {
            args.push(attr("label", label));
        }
        for (key, a) in &edge.attributes {
            match key.as_str() {
                "source-side" | "target-side" | "via" | "tone" => args.push(Arg::Attr(
                    match key.as_str() {
                        "source-side" => "from",
                        "target-side" => "to",
                        other => other,
                    }
                    .into(),
                    old(&a.value, a.span)?,
                )),
                "stroke" => match text(&a.value, a.span)?.as_str() {
                    "dashed" => args.push(Arg::Value(OldValue::Ident("dashed".into()))),
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
        evidence.authored_id = edge.id.clone();
        self.relationships.insert(id, evidence);
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
            from_span: edge.span,
            to_span: edge.span,
            arrow_span: edge.span,
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

fn lower(document: &Document, options: &CompileOptions) -> Result<Lowered, Error> {
    if options.view.is_some() {
        return Err(fail(
            document.diagrams[0].span,
            "document/view",
            "views are not implemented in revision-one documents yet",
        ));
    }
    let mut outputs = Vec::new();
    for diagram in &document.diagrams {
        allowed(
            &diagram.attributes,
            &[
                "kind",
                "width",
                "layout",
                "direction",
                "textdir",
                "preset",
                "subtitle",
            ],
        )?;
        let Body::Graph(body) = &diagram.body;
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
        };
        builder.collect(body, &[], true)?;
        builder.validate_shadowing()?;
        // Validate unused kind declarations too, before selection/layout.
        let mut validated_styles = Vec::new();
        for (edge, kinds) in [(false, &builder.node_kinds), (true, &builder.edge_kinds)] {
            for (name, kind) in kinds {
                allowed(
                    &kind.attributes,
                    if edge {
                        &["base", "tone", "stroke", "label"]
                    } else {
                        &["base", "tone", "fill", "font", "align", "role", "shape"]
                    },
                )?;
                let (base, attrs) = builder.kind(name, edge, kind.span, &mut BTreeSet::new())?;
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
        let mut body = builder.lower(body, &[])?;
        let mut args = vec![attr("title", &diagram.title)];
        for (key, a) in &diagram.attributes {
            if key == "kind" || key == "subtitle" {
                continue;
            }
            args.push(Arg::Attr(
                if key == "textdir" {
                    "text-direction".into()
                } else {
                    key.clone()
                },
                old(&a.value, a.span)?,
            ));
        }
        let evidence = annotations(&diagram.annotations)?;
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
                    "note",
                    vec![Arg::Value(OldValue::Str(subtitle))],
                    None,
                    diagram.span,
                ),
            );
        }
        body.insert(
            0,
            item(
                "legend",
                vec![Arg::Value(OldValue::Ident("off".into()))],
                None,
                diagram.span,
            ),
        );
        builder.styles.extend(body);
        let statements = vec![item("diagram", args, Some(builder.styles), diagram.span)];
        // Validate every diagram before choosing one, including unused styles.
        let mut model = crate::model::build_statements_with_tags(&statements, false)?;
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
                diagram: evidence,
                objects: builder.objects,
                relationships: builder.relationships,
            },
            semantic_kinds: builder.semantic_kinds,
        });
    }
    let index = match &options.diagram {
        Some(id) => document
            .diagrams
            .iter()
            .position(|d| &d.id == id)
            .ok_or_else(|| {
                fail(
                    document.diagrams[0].span,
                    "document/diagram",
                    format!("unknown diagram `{id}`"),
                )
            })?,
        None => 0,
    };
    Ok(outputs.swap_remove(index))
}

pub(super) fn build(document: &Document) -> Result<crate::model::Diagram, Error> {
    let mut lowered = lower(document, &CompileOptions::default())?;
    restore_kinds(&mut lowered.diagram.blocks, &lowered.semantic_kinds);
    Ok(lowered.diagram)
}

fn restore_kinds(blocks: &mut [crate::model::Block], kinds: &BTreeMap<String, String>) {
    use crate::model::Block;
    for block in blocks {
        match block {
            Block::Node(node) => {
                if let Some(kind) = kinds.get(&node.id) {
                    node.kind = kind.clone();
                }
                restore_kinds(&mut node.children, kinds);
            }
            Block::Row(row) => {
                for cell in row.cells.iter_mut().flatten() {
                    restore_kinds(std::slice::from_mut(cell), kinds);
                }
            }
            Block::Section(section) => restore_kinds(&mut section.children, kinds),
            Block::Tree { nodes, .. } => restore_kinds(nodes, kinds),
            Block::Flow { layers, .. } => {
                for layer in layers {
                    restore_kinds(layer, kinds);
                }
            }
            _ => {}
        }
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
    } = lower(document, options)?;
    let mut compiled =
        crate::compile_lowered(&statements, &CompileOptions::default(), fonts, false)?;
    restore_kinds(&mut compiled.diagram.blocks, &semantic_kinds);
    compiled.diagram.kinds = diagram.kinds;
    for node in &mut compiled.scene.nodes {
        if let Some(kind) = semantic_kinds.get(&node.id) {
            node.kind = kind.clone();
        }
    }
    compiled.document = Some(info);
    Ok(compiled)
}
