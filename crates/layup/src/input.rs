//! Versioned semantic graph input for analyzers. Lower to the same statement
//! pipeline as the DSL without encoding symbol IDs or inventing source spans.
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as Json};

use crate::{
    CompileOptions, Compiled, Error,
    diagnostic::Span,
    parser::{Arg, EdgeStmt, Item, Stmt, Value},
    text::Fonts,
};

pub const VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Graph {
    pub version: u32,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub direction: Direction,
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub edges: Vec<Edge>,
    #[serde(default)]
    pub views: Vec<View>,
    #[serde(default)]
    pub provenance: Option<Provenance>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    #[default]
    Down,
    Up,
    Right,
    Left,
}
impl Direction {
    fn name(self) -> &'static str {
        match self {
            Self::Down => "down",
            Self::Up => "up",
            Self::Right => "right",
            Self::Left => "left",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Node {
    pub id: String,
    pub title: String,
    #[serde(default = "node_kind")]
    pub kind: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub code: Vec<String>,
    #[serde(default)]
    pub description: Vec<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub href: Option<String>,
    #[serde(default)]
    pub tone: Option<String>,
    #[serde(default)]
    pub source_locations: Vec<SourceLocation>,
    #[serde(default)]
    pub metadata: Map<String, Json>,
}
fn node_kind() -> String {
    "node".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Edge {
    pub id: String,
    pub from: String,
    pub to: String,
    #[serde(default = "edge_kind")]
    pub kind: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub tone: Option<String>,
    #[serde(default)]
    pub dashed: Option<bool>,
    #[serde(default)]
    pub source_locations: Vec<SourceLocation>,
    #[serde(default)]
    pub metadata: Map<String, Json>,
}
fn edge_kind() -> String {
    "flow".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct View {
    pub id: String,
    pub title: String,
    pub include: Vec<String>,
    #[serde(default)]
    pub direction: Option<Direction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Provenance {
    pub analyzer: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub metadata: Map<String, Json>,
}

/// URI relative to the analyzer's source root, or an absolute URI. A missing
/// range means only the file is known; adapters must not guess a line number.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceLocation {
    pub uri: String,
    #[serde(default)]
    pub range: Option<SourceRange>,
    #[serde(default)]
    pub symbol: Option<String>,
}

/// 1-based lines and Unicode scalar columns; end position is exclusive.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceRange {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

pub fn parse(json: &str) -> Result<Graph, Error> {
    serde_json::from_str(json).map_err(|error| invalid(format!("invalid graph JSON: {error}")))
}

pub fn compile_json(
    json: &str,
    options: &CompileOptions,
    fonts: &Fonts,
) -> Result<Compiled, Error> {
    compile(&parse(json)?, options, fonts)
}

pub fn compile(graph: &Graph, options: &CompileOptions, fonts: &Fonts) -> Result<Compiled, Error> {
    if options.diagram.is_some() {
        return Err(Error::new(
            "diagram selection requires a `layup 1` document",
        ));
    }
    let statements = graph.statements()?;
    let mut compiled =
        crate::compile_statements(&statements, options, fonts).map_err(|mut error| {
            error.line = None;
            error.span = None;
            error.related.clear();
            error
        })?;
    for warning in &mut compiled.warnings {
        warning.line = None;
    }
    compiled.input = Some(graph.clone());
    Ok(compiled)
}

fn invalid(message: impl Into<String>) -> Error {
    let mut error = Error::new(message);
    error.code = "input/graph";
    error
}

fn attr(key: &str, value: &str) -> Arg {
    Arg::Attr(key.into(), Value::Str(value.into()))
}
fn item(head: &str, args: Vec<Arg>, body: Option<Vec<Stmt>>) -> Stmt {
    Stmt::Item(Item {
        head: head.into(),
        arg_spans: vec![Span::default(); args.len()],
        args,
        body,
        line: 0,
        span: Span::default(),
        head_span: Span::default(),
    })
}
fn text(head: &str, value: &str) -> Stmt {
    item(head, vec![Arg::Value(Value::Str(value.into()))], None)
}

fn kind(name: &str, node: bool) -> Result<(), Error> {
    let tokens = crate::lexer::lex(name).map_err(|_| invalid(format!("invalid kind `{name}`")))?;
    if !matches!(tokens.as_slice(), [token, end] if matches!(&token.tok, crate::lexer::Tok::Ident(id) if id == name) && matches!(end.tok, crate::lexer::Tok::Newline))
        || node
            && matches!(
                name,
                "diagram"
                    | "model"
                    | "view"
                    | "row"
                    | "section"
                    | "band"
                    | "text"
                    | "divider"
                    | "gap"
                    | "title"
                    | "note"
                    | "subtitle"
                    | "desc"
                    | "width"
                    | "preset"
                    | "style"
                    | "arrow"
                    | "legend"
                    | "code"
                    | "sub"
                    | "name"
                    | "role"
                    | "tag"
                    | "step"
                    | "loop"
                    | "opt"
                    | "alt"
                    | "branch"
                    | "participant"
                    | "actor"
            )
    {
        return Err(invalid(format!("unsupported graph kind `{name}`")));
    }
    Ok(())
}

fn locations(id: &str, values: &[SourceLocation]) -> Result<(), Error> {
    for location in values {
        if location.uri.trim().is_empty() {
            return Err(invalid(format!("`{id}` has an empty source URI")));
        }
        if let Some(range) = &location.range
            && ([
                range.start_line,
                range.start_column,
                range.end_line,
                range.end_column,
            ]
            .contains(&0)
                || (range.end_line, range.end_column) < (range.start_line, range.start_column))
        {
            return Err(invalid(format!("`{id}` has an invalid source range")));
        }
    }
    Ok(())
}

fn tone(id: &str, value: &Option<String>) -> Result<(), Error> {
    if let Some(value) = value
        && crate::style::Tone::parse(value).is_none()
    {
        return Err(invalid(format!("`{id}` has unknown tone `{value}`")));
    }
    Ok(())
}

impl Graph {
    fn statements(&self) -> Result<Vec<Stmt>, Error> {
        if self.version != VERSION {
            return Err(invalid(format!(
                "unsupported graph version {}; expected {VERSION}",
                self.version
            )));
        }
        if self.title.trim().is_empty() {
            return Err(invalid("graph title cannot be empty"));
        }
        let mut nodes = BTreeMap::new();
        let mut children: BTreeMap<Option<&str>, Vec<&Node>> = BTreeMap::new();
        for node in &self.nodes {
            if node.id.is_empty() || nodes.insert(node.id.as_str(), node).is_some() {
                return Err(invalid(format!("empty or duplicate node ID `{}`", node.id)));
            }
            kind(&node.kind, true)?;
            tone(&node.id, &node.tone)?;
            locations(&node.id, &node.source_locations)?;
            children
                .entry(node.parent_id.as_deref())
                .or_default()
                .push(node);
        }
        for node in &self.nodes {
            let mut path = BTreeSet::from([node.id.as_str()]);
            let mut parent = node.parent_id.as_deref();
            while let Some(id) = parent {
                let ancestor = nodes.get(id).ok_or_else(|| {
                    invalid(format!("node `{}` has unknown parent `{id}`", node.id))
                })?;
                if !path.insert(id) {
                    return Err(invalid(format!("parent cycle involving `{}`", node.id)));
                }
                if path.len() > 128 {
                    return Err(invalid("graph hierarchy may nest at most 128 levels"));
                }
                parent = ancestor.parent_id.as_deref();
            }
        }
        let mut edge_ids = BTreeSet::new();
        for edge in &self.edges {
            if edge.id.is_empty() || !edge_ids.insert(&edge.id) {
                return Err(invalid(format!("empty or duplicate edge ID `{}`", edge.id)));
            }
            if !nodes.contains_key(edge.from.as_str()) || !nodes.contains_key(edge.to.as_str()) {
                return Err(invalid(format!(
                    "edge `{}` refers to an unknown node",
                    edge.id
                )));
            }
            kind(&edge.kind, false)?;
            tone(&edge.id, &edge.tone)?;
            locations(&edge.id, &edge.source_locations)?;
        }
        let mut view_ids = BTreeSet::new();
        for view in &self.views {
            if view.id.is_empty() || !view_ids.insert(&view.id) {
                return Err(invalid(format!("empty or duplicate view ID `{}`", view.id)));
            }
            if view.title.trim().is_empty() || view.include.is_empty() {
                return Err(invalid(format!(
                    "view `{}` needs a title and included nodes",
                    view.id
                )));
            }
            let mut included = BTreeSet::new();
            for id in &view.include {
                if !nodes.contains_key(id.as_str()) || !included.insert(id) {
                    return Err(invalid(format!(
                        "view `{}` has unknown or duplicate include `{id}`",
                        view.id
                    )));
                }
            }
        }
        let mut body = Vec::new();
        if let Some(description) = &self.description {
            body.push(text("desc", description));
        }
        let presets = crate::style::presets();
        for name in self.nodes.iter().map(|n| &n.kind).collect::<BTreeSet<_>>() {
            if !presets.contains_key(name) {
                body.push(item(
                    "style",
                    vec![Arg::Value(Value::Ident(name.clone())), attr("base", "node")],
                    None,
                ));
            }
        }
        let arrows = crate::style::arrow_presets();
        for name in self.edges.iter().map(|e| &e.kind).collect::<BTreeSet<_>>() {
            if !arrows.contains_key(name) {
                body.push(item(
                    "arrow",
                    vec![
                        Arg::Value(Value::Ident(name.clone())),
                        Arg::Value(Value::Ident("blue".into())),
                        Arg::Value(Value::Str(name.clone())),
                    ],
                    None,
                ));
            }
        }
        fn lower(node: &Node, children: &BTreeMap<Option<&str>, Vec<&Node>>) -> Stmt {
            let mut args = vec![
                attr("id", &node.id),
                Arg::Value(Value::Str(node.title.clone())),
            ];
            for (key, value) in [
                ("role", &node.role),
                ("href", &node.href),
                ("tone", &node.tone),
            ] {
                if let Some(value) = value {
                    args.push(attr(key, value));
                }
            }
            let mut body: Vec<_> = node
                .code
                .iter()
                .map(|s| text("code", s))
                .chain(node.description.iter().map(|s| text("sub", s)))
                .collect();
            if let Some(nested) = children.get(&Some(node.id.as_str())) {
                body.extend(nested.iter().map(|n| lower(n, children)));
            }
            item(
                &node.kind,
                args,
                if body.is_empty() { None } else { Some(body) },
            )
        }
        if let Some(roots) = children.get(&None) {
            body.extend(roots.iter().map(|n| lower(n, &children)));
        }
        for edge in &self.edges {
            let mut args = vec![attr("id", &edge.id)];
            if let Some(label) = &edge.label {
                args.push(Arg::Value(Value::Str(label.clone())));
            }
            if let Some(tone) = &edge.tone {
                args.push(attr("tone", tone));
            }
            if let Some(dashed) = edge.dashed {
                args.push(Arg::Value(Value::Ident(
                    if dashed { "dashed" } else { "solid" }.into(),
                )));
            }
            body.push(Stmt::Edge(EdgeStmt {
                from: edge.from.clone(),
                to: edge.to.clone(),
                kind: Some(edge.kind.clone()),
                left: false,
                right: true,
                args: args.clone(),
                line: 0,
                span: Span::default(),
                from_span: Span::default(),
                to_span: Span::default(),
                arrow_span: Span::default(),
                arg_spans: vec![Span::default(); args.len()],
            }));
        }
        for view in &self.views {
            let mut args = vec![
                Arg::Value(Value::Ident(view.id.clone())),
                Arg::Value(Value::Str(view.title.clone())),
            ];
            if let Some(direction) = view.direction {
                args.push(attr("direction", direction.name()));
            }
            body.push(item(
                "view",
                args,
                Some(vec![item(
                    "include",
                    view.include
                        .iter()
                        .map(|id| Arg::Value(Value::Str(id.clone())))
                        .collect(),
                    None,
                )]),
            ));
        }
        Ok(vec![item(
            if self.views.is_empty() {
                "diagram"
            } else {
                "model"
            },
            vec![
                Arg::Value(Value::Str(self.title.clone())),
                attr("layout", "auto"),
                attr("direction", self.direction.name()),
            ],
            Some(body),
        )])
    }

    /// Analysis evidence for the selected scene, also embedded in SVG/HTML.
    pub(crate) fn analysis(&self, compiled: &Compiled) -> Json {
        let nodes: BTreeSet<_> = compiled.scene.nodes.iter().map(|n| n.id.as_str()).collect();
        let edges: BTreeSet<_> = compiled.scene.edges.iter().map(|e| e.id.as_str()).collect();
        serde_json::json!({ "version": VERSION, "provenance": self.provenance,
            "nodes": self.nodes.iter().filter(|n| nodes.contains(n.id.as_str())).map(|n| serde_json::json!({"id": n.id, "sourceLocations": n.source_locations, "metadata": n.metadata})).collect::<Vec<_>>(),
            "edges": self.edges.iter().filter(|e| edges.contains(e.id.as_str())).map(|e| serde_json::json!({"id": e.id, "sourceLocations": e.source_locations, "metadata": e.metadata})).collect::<Vec<_>>() })
    }
}
