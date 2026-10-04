//! Authored, cumulative presentation steps. Static rendering remains complete;
//! interactive viewers opt in to the resolved visibility plan.
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    Error,
    diagnostic::{Span, quote},
    layout::Scene,
    parser::{Arg, Item, Stmt, Value},
};

#[derive(Debug, Clone)]
pub struct Target {
    pub id: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StepDefinition {
    pub id: String,
    pub title: String,
    pub note: Option<String>,
    pub show: Vec<Target>,
    pub show_edges: Vec<Target>,
    pub highlight: Vec<Target>,
    pub highlight_edges: Vec<Target>,
    pub span: Span,
}

#[derive(Debug, Clone, Default)]
pub struct Presentation {
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone)]
pub struct Step {
    pub id: String,
    pub title: String,
    pub note: Option<String>,
    pub visible_nodes: Vec<String>,
    pub visible_edges: Vec<String>,
    pub highlight_nodes: Vec<String>,
    pub highlight_edges: Vec<String>,
}

impl Presentation {
    /// A versioned, renderer-independent plan. Serialize before HTML escaping.
    pub fn json(&self) -> String {
        let steps = self.steps.iter().map(|s| format!(
            "{{\"id\":{},\"title\":{},\"note\":{},\"visibleNodes\":{},\"visibleEdges\":{},\"highlightNodes\":{},\"highlightEdges\":{}}}",
            quote(&s.id), quote(&s.title), s.note.as_deref().map_or("null".into(), quote),
            strings(&s.visible_nodes), strings(&s.visible_edges), strings(&s.highlight_nodes), strings(&s.highlight_edges)
        )).collect::<Vec<_>>().join(",");
        format!("{{\"version\":1,\"steps\":[{steps}]}}")
    }
}

fn strings(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|s| quote(s))
            .collect::<Vec<_>>()
            .join(",")
    )
}

/// Remove direct diagram steps before semantic model building. A wrapperless
/// document can also carry steps at its root; nested steps are rejected.
pub fn extract(statements: &mut Vec<Stmt>) -> Result<Vec<StepDefinition>, Error> {
    let mut steps = Vec::new();
    extract_scope(statements, true, &mut steps)?;
    let mut ids = BTreeMap::new();
    for step in &steps {
        if let Some(first) = ids.insert(step.id.clone(), step.span) {
            return Err(Error::located(
                step.span,
                "presentation/duplicate-step",
                format!("duplicate presentation step `{}`", step.id),
            )
            .with_related(first, "first step with this name"));
        }
    }
    Ok(steps)
}

fn extract_scope(
    statements: &mut Vec<Stmt>,
    allowed: bool,
    steps: &mut Vec<StepDefinition>,
) -> Result<(), Error> {
    let mut ordinary = Vec::new();
    for mut statement in std::mem::take(statements) {
        if let Stmt::Item(item) = &mut statement {
            if item.head == "step" {
                if !allowed {
                    return Err(Error::located(
                        item.head_span,
                        "presentation/nested-step",
                        "presentation steps must be direct children of the diagram",
                    ));
                }
                steps.push(parse_step(item)?);
                continue;
            }
            if let Some(body) = &mut item.body {
                extract_scope(body, allowed && item.head == "diagram", steps)?;
            }
        }
        ordinary.push(statement);
    }
    *statements = ordinary;
    Ok(())
}

fn parse_step(item: &Item) -> Result<StepDefinition, Error> {
    let id = match item.args.first() {
        Some(Arg::Value(Value::Ident(id))) if !id.is_empty() => id.clone(),
        _ => {
            return Err(Error::located(
                item.head_span,
                "presentation/step-id",
                "step needs a name, for example `step overview \"Overview\" { show api }`",
            ));
        }
    };
    let title = match item.args.get(1) {
        Some(Arg::Value(Value::Str(title))) => title.clone(),
        None => id.clone(),
        _ => {
            return Err(Error::located(
                item.arg_spans[1],
                "presentation/step-title",
                "step title must be a quoted string",
            ));
        }
    };
    if item.args.len() > 2 {
        return Err(Error::located(
            item.arg_spans[2],
            "presentation/step-argument",
            "unexpected step argument",
        ));
    }
    let body = item.body.as_ref().ok_or_else(|| Error::located(item.head_span, "presentation/step-body", "step needs a body containing show, show-edge, highlight, highlight-edge, or note directives"))?;
    let mut out = StepDefinition {
        id,
        title,
        note: None,
        show: Vec::new(),
        show_edges: Vec::new(),
        highlight: Vec::new(),
        highlight_edges: Vec::new(),
        span: item.arg_spans[0],
    };
    for statement in body {
        let Stmt::Item(directive) = statement else {
            return Err(Error::located(
                statement.span(),
                "presentation/directive",
                "edges are declared outside presentation steps; use `show-edge edge-1` to reveal one",
            ));
        };
        if directive.body.is_some() {
            return Err(Error::located(
                directive.head_span,
                "presentation/directive-body",
                "presentation directives cannot have nested bodies",
            ));
        }
        let target = match directive.head.as_str() {
            "show" => &mut out.show,
            "show-edge" => &mut out.show_edges,
            "highlight" => &mut out.highlight,
            "highlight-edge" => &mut out.highlight_edges,
            "note" => {
                if out.note.is_some() {
                    return Err(Error::located(
                        directive.head_span,
                        "presentation/duplicate-note",
                        "a step can have only one note",
                    ));
                }
                match directive.args.as_slice() {
                    [Arg::Value(Value::Str(note))] => out.note = Some(note.clone()),
                    _ => {
                        return Err(Error::located(
                            directive.head_span,
                            "presentation/note",
                            "step note needs one quoted string",
                        ));
                    }
                }
                continue;
            }
            _ => {
                return Err(Error::located(
                    directive.head_span,
                    "presentation/directive",
                    format!("unknown presentation directive `{}`", directive.head),
                )
                .with_help("use show, show-edge, highlight, highlight-edge, or note"));
            }
        };
        if directive.args.is_empty() {
            return Err(Error::located(
                directive.head_span,
                "presentation/target",
                format!("{} needs at least one target", directive.head),
            ));
        }
        for (arg, span) in directive.args.iter().zip(&directive.arg_spans) {
            match arg {
                Arg::Value(Value::Ident(id) | Value::Str(id)) => target.push(Target {
                    id: id.clone(),
                    span: *span,
                }),
                _ => {
                    return Err(Error::located(
                        *span,
                        "presentation/target",
                        "presentation targets must be node IDs or edge IDs",
                    ));
                }
            }
        }
    }
    Ok(out)
}

/// Resolve authored targets against the finished scene, including container
/// ancestry. Edges use their stable authored IDs or generated `edge-N` IDs.
pub fn resolve(definitions: Vec<StepDefinition>, scene: &Scene) -> Result<Presentation, Error> {
    let node_ids: BTreeMap<_, _> = scene
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    let edge_ids: BTreeMap<_, _> = scene
        .edges
        .iter()
        .enumerate()
        .map(|(i, e)| (e.id.clone(), i))
        .collect();
    let explicit_edges: BTreeSet<_> = definitions
        .iter()
        .flat_map(|s| s.show_edges.iter().map(|t| t.id.as_str()))
        .collect();
    let mut visible_nodes = BTreeSet::new();
    let mut assigned_edges = BTreeSet::new();
    let mut presentation = Presentation::default();
    for definition in &definitions {
        for target in definition.show.iter().chain(&definition.highlight) {
            if !node_ids.contains_key(target.id.as_str()) {
                return Err(Error::located(
                    target.span,
                    "presentation/node",
                    format!("unknown presentation node `{}`", target.id),
                ));
            }
        }
        for target in definition
            .show_edges
            .iter()
            .chain(&definition.highlight_edges)
        {
            if !edge_ids.contains_key(&target.id) {
                return Err(Error::located(
                    target.span,
                    "presentation/edge",
                    format!("unknown presentation edge `{}`", target.id),
                )
                .with_help("use the edge id= attribute or generated edge-1, edge-2, ... IDs"));
            }
        }
        for target in &definition.show {
            let i = node_ids[target.id.as_str()];
            visible_nodes.insert(i);
            visible_nodes.extend(scene.ancestors(i));
            visible_nodes.extend((0..scene.nodes.len()).filter(|&n| scene.is_descendant(n, i)));
        }
        for target in &definition.show_edges {
            let i = edge_ids[&target.id];
            let edge = &scene.edges[i];
            if !visible_nodes.contains(&node_ids[edge.from.as_str()])
                || !visible_nodes.contains(&node_ids[edge.to.as_str()])
            {
                return Err(Error::located(
                    target.span,
                    "presentation/hidden-endpoint",
                    format!("show both endpoints before revealing `{}`", target.id),
                ));
            }
            assigned_edges.insert(i);
        }
        let visible_edges: BTreeSet<_> = scene
            .edges
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let both = visible_nodes.contains(&node_ids[e.from.as_str()])
                    && visible_nodes.contains(&node_ids[e.to.as_str()]);
                (both && (!explicit_edges.contains(e.id.as_str()) || assigned_edges.contains(&i)))
                    .then_some(i)
            })
            .collect();
        for target in &definition.highlight {
            if !visible_nodes.contains(&node_ids[target.id.as_str()]) {
                return Err(Error::located(
                    target.span,
                    "presentation/hidden-highlight",
                    format!("show node `{}` before highlighting it", target.id),
                ));
            }
        }
        for target in &definition.highlight_edges {
            if !visible_edges.contains(&edge_ids[&target.id]) {
                return Err(Error::located(
                    target.span,
                    "presentation/hidden-highlight",
                    format!("show edge `{}` before highlighting it", target.id),
                ));
            }
        }
        presentation.steps.push(Step {
            id: definition.id.clone(),
            title: definition.title.clone(),
            note: definition.note.clone(),
            visible_nodes: visible_nodes
                .iter()
                .map(|&i| scene.nodes[i].id.clone())
                .collect(),
            visible_edges: visible_edges
                .iter()
                .map(|&i| scene.edges[i].id.clone())
                .collect(),
            highlight_nodes: definition
                .highlight
                .iter()
                .map(|t| t.id.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            highlight_edges: definition
                .highlight_edges
                .iter()
                .map(|t| t.id.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        });
    }
    Ok(presentation)
}
