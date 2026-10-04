//! Resolve named views of a shared model before ordinary semantic compilation.
//!
//! Shared model nodes require explicit IDs: generated IDs must not depend on
//! which view happens to be rendered. Selecting a container selects its entire
//! subtree; selecting a descendant retains its ancestor containers. References
//! are never rewritten or retargeted.
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    Error,
    diagnostic::Span,
    parser::{Arg, Item, Stmt, Value},
    style::{Tone, presets},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewInfo {
    pub id: String,
    pub title: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Resolved {
    pub statements: Vec<Stmt>,
    pub views: Vec<ViewInfo>,
    pub selected_view: Option<String>,
}

/// Ordinary diagrams pass through unchanged. A model contains shared diagram
/// statements and one or more `view id "Title" { include node-id ... }` blocks.
/// The first authored view is the default when no selector is supplied.
pub fn resolve(statements: &[Stmt], selector: Option<&str>) -> Result<Resolved, Error> {
    let models: Vec<_> = statements
        .iter()
        .filter_map(|s| match s {
            Stmt::Item(item) if item.head == "model" => Some(item),
            _ => None,
        })
        .collect();
    if models.is_empty() {
        if let Some(selector) = selector {
            return Err(Error::new(format!(
                "view `{selector}` cannot be selected: this document has no model"
            ))
            .with_help("define a `model` with named `view` blocks, or omit the view selector"));
        }
        return Ok(Resolved {
            statements: statements.to_vec(),
            views: Vec::new(),
            selected_view: None,
        });
    }
    if models.len() > 1 {
        return Err(Error::located(
            models[1].head_span,
            "view/model-count",
            "a document may contain only one shared model",
        )
        .with_related(models[0].head_span, "the first model is here"));
    }
    let model = models[0];
    if statements.len() != 1 {
        let extra = statements
            .iter()
            .find(|s| !matches!(s, Stmt::Item(i) if i.head == "model"))
            .expect("the document has extra statements");
        return Err(Error::located(
            extra.span(),
            "view/model-document",
            "shared statements and named views must be inside the model block",
        ));
    }
    let model_title = root_title(model)?;
    let body = model.body.as_deref().ok_or_else(|| {
        Error::located(model.head_span, "view/model-body", "a model needs a body")
    })?;
    let mut kinds: BTreeSet<String> = presets().into_keys().collect();
    // Sequence participants also have stable explicit IDs; sequence-specific
    // validation remains in its semantic compiler.
    kinds.extend(["participant".into(), "actor".into()]);
    for stmt in body {
        if let Stmt::Item(item) = stmt
            && item.head == "style"
            && let Some(Arg::Value(Value::Ident(name))) = item.args.first()
        {
            kinds.insert(name.clone());
        }
    }
    // The ordinary compiler gives structural and metadata statements
    // precedence over custom style names, even if a style shares their name.
    kinds.retain(|kind| {
        !matches!(
            kind.as_str(),
            "row"
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
                | "model"
                | "view"
                | "diagram"
                | "loop"
                | "opt"
                | "alt"
                | "branch"
        )
    });
    let mut shared: Vec<_> = body
        .iter()
        .filter(|s| !matches!(s, Stmt::Item(i) if i.head == "view"))
        .cloned()
        .collect();
    assign_edge_ids(&mut shared, &kinds)?;
    let mut nodes = BTreeMap::new();
    index_nodes(&shared, &kinds, &mut nodes)?;
    validate_edges(&shared, &nodes)?;
    let mut view_items = Vec::new();
    let mut infos: Vec<ViewInfo> = Vec::new();
    let mut selections = Vec::new();
    for stmt in body {
        let Stmt::Item(view) = stmt else { continue };
        if view.head != "view" {
            continue;
        }
        let info = view_info(view, &model_title)?;
        if let Some(first) = infos.iter().find(|first| first.id == info.id) {
            return Err(Error::located(
                view.arg_spans.first().copied().unwrap_or(view.head_span),
                "view/duplicate-id",
                format!("duplicate view ID `{}`", info.id),
            )
            .with_related(first.span, "the first view is here"));
        }
        let selection = selection(view, &nodes)?;
        infos.push(info);
        view_items.push(view);
        selections.push(selection);
    }
    if infos.is_empty() {
        return Err(Error::located(
            model.head_span,
            "view/missing",
            "a shared model needs at least one named view",
        )
        .with_help("add `view overview \"Overview\" { include node-id }`"));
    }
    let index = match selector {
        None => 0,
        Some(id) => infos.iter().position(|v| v.id == id).ok_or_else(|| {
            Error::located(
                model.head_span,
                "view/unknown",
                format!("unknown view `{id}`"),
            )
            .with_optional_help(crate::diagnostic::suggestion(
                id,
                infos.iter().map(|v| v.id.as_str()),
            ))
        })?,
    };
    let view = view_items[index];
    let mut root = model.clone();
    root.head = "diagram".into();
    root.args = vec![Arg::Value(Value::Str(infos[index].title.clone()))];
    root.arg_spans = vec![view.arg_spans.get(1).copied().unwrap_or(view.head_span)];
    // Keep the source location of inherited options, replacing it with the
    // selected view's location only when that view supplies an override.
    for item in [model, view] {
        for (arg, span) in item.args.iter().zip(&item.arg_spans) {
            if let Arg::Attr(key, _) = arg {
                if key == "title" {
                    continue;
                }
                if let Some(pos) = root
                    .args
                    .iter()
                    .position(|a| matches!(a, Arg::Attr(k, _) if k == key))
                {
                    root.args[pos] = arg.clone();
                    root.arg_spans[pos] = *span;
                } else {
                    root.args.push(arg.clone());
                    root.arg_spans.push(*span);
                }
            }
        }
    }
    let sequence = root.args.iter().any(|arg| matches!(arg, Arg::Attr(key, value) if key == "mode" && value.as_text() == "sequence"));
    if sequence {
        validate_sequence_notes(&shared, &nodes)?;
    }
    let mut filtered = filter(&shared, &selections[index], &kinds, sequence);
    // A view title overrides a shared title directive; all other metadata
    // remains authored in order. Steps can be shared or replaced per view.
    filtered.retain(|s| !matches!(s, Stmt::Item(i) if i.head == "title"));
    let steps: Vec<_> = view
        .body
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .filter(|s| matches!(s, Stmt::Item(i) if i.head == "step"))
        .cloned()
        .collect();
    if !steps.is_empty() {
        filtered.retain(|s| !matches!(s, Stmt::Item(i) if i.head == "step"));
        filtered.extend(steps);
    }
    root.body = Some(filtered);
    Ok(Resolved {
        statements: vec![Stmt::Item(root)],
        selected_view: Some(infos[index].id.clone()),
        views: infos,
    })
}

fn root_title(item: &Item) -> Result<String, Error> {
    let mut title = None;
    for (arg, span) in item.args.iter().zip(&item.arg_spans) {
        match arg {
            Arg::Value(Value::Str(value)) if title.is_none() => title = Some(value.clone()),
            Arg::Attr(key, value) if key == "title" => title = Some(value.as_text()),
            Arg::Attr(_, _) => {}
            _ => {
                return Err(Error::located(
                    *span,
                    "view/model-argument",
                    "a model takes one quoted title and diagram attributes",
                ));
            }
        }
    }
    title.filter(|s| !s.is_empty()).ok_or_else(|| {
        Error::located(
            item.head_span,
            "view/model-title",
            "a model needs a nonempty quoted title",
        )
    })
}

fn view_info(item: &Item, fallback: &str) -> Result<ViewInfo, Error> {
    let Some(Arg::Value(Value::Ident(id))) = item.args.first() else {
        return Err(Error::located(
            item.head_span,
            "view/id",
            "a view needs an ID before its optional quoted title",
        ));
    };
    let mut title = fallback.to_string();
    let mut title_set = false;
    for (arg, span) in item.args.iter().zip(&item.arg_spans).skip(1) {
        match arg {
            Arg::Value(Value::Str(value)) if !title_set => {
                title = value.clone();
                title_set = true;
            }
            Arg::Attr(key, value) if key == "title" => title = value.as_text(),
            Arg::Attr(_, _) => {}
            _ => {
                return Err(Error::located(
                    *span,
                    "view/argument",
                    "a view takes an ID, optional quoted title, and diagram attributes",
                ));
            }
        }
    }
    if title.is_empty() {
        return Err(Error::located(
            item.head_span,
            "view/title",
            "a view title cannot be empty",
        ));
    }
    Ok(ViewInfo {
        id: id.clone(),
        title,
        span: item.span,
    })
}

fn explicit_id(item: &Item) -> Option<(String, Span)> {
    let mut id = None;
    for (arg, span) in item.args.iter().zip(&item.arg_spans) {
        match arg {
            Arg::Attr(key, value) if key == "id" => id = Some((value.as_text(), *span)),
            Arg::Value(Value::Ident(word))
                if id.is_none()
                    && Tone::parse(word).is_none()
                    && !matches!(
                        word.as_str(),
                        "hollow"
                            | "filled"
                            | "center"
                            | "left"
                            | "mono"
                            | "sans"
                            | "dashed"
                            | "solid"
                            | "inherit"
                            | "top"
                            | "bottom"
                            | "auto"
                            | "caps"
                    ) =>
            {
                id = Some((word.clone(), *span))
            }
            _ => {}
        }
    }
    id
}

// Assign identities before endpoint filtering, so an edge keeps the same
// identity in every view even when earlier edges are omitted from that view.
fn assign_edge_ids(stmts: &mut [Stmt], kinds: &BTreeSet<String>) -> Result<(), Error> {
    fn reserve(
        stmts: &[Stmt],
        kinds: &BTreeSet<String>,
        ids: &mut BTreeMap<String, Span>,
    ) -> Result<(), Error> {
        for stmt in stmts {
            match stmt {
                Stmt::Edge(edge) => {
                    for (arg, span) in edge.args.iter().zip(&edge.arg_spans) {
                        if let Arg::Attr(key, value) = arg
                            && key == "id"
                        {
                            let id = value.as_text();
                            if id.is_empty() {
                                continue;
                            }
                            if let Some(first) = ids.insert(id.clone(), *span) {
                                return Err(Error::located(
                                    *span,
                                    "semantic/duplicate-edge-id",
                                    format!("duplicate edge id `{id}`"),
                                )
                                .with_related(first, "first edge with this id"));
                            }
                        }
                    }
                }
                Stmt::Item(item)
                    if kinds.contains(&item.head)
                        || matches!(
                            item.head.as_str(),
                            "row" | "section" | "band" | "loop" | "opt" | "alt" | "branch"
                        ) =>
                {
                    if let Some(body) = &item.body {
                        reserve(body, kinds, ids)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn assign(
        stmts: &mut [Stmt],
        kinds: &BTreeSet<String>,
        ids: &mut BTreeSet<String>,
        next: &mut usize,
    ) {
        for stmt in stmts {
            match stmt {
                Stmt::Edge(edge) => {
                    let index = edge
                        .args
                        .iter()
                        .position(|a| matches!(a, Arg::Attr(key, _) if key == "id"));
                    if index.is_some_and(|i| matches!(&edge.args[i], Arg::Attr(_, value) if !value.as_text().is_empty())) { continue; }
                    while ids.contains(&format!("edge-{}", *next)) {
                        *next += 1;
                    }
                    let id = format!("edge-{}", *next);
                    ids.insert(id.clone());
                    *next += 1;
                    let arg = Arg::Attr("id".into(), Value::Str(id));
                    if let Some(index) = index {
                        edge.args[index] = arg;
                    } else {
                        edge.args.push(arg);
                        edge.arg_spans.push(edge.span);
                    }
                }
                Stmt::Item(item)
                    if kinds.contains(&item.head)
                        || matches!(
                            item.head.as_str(),
                            "row" | "section" | "band" | "loop" | "opt" | "alt" | "branch"
                        ) =>
                {
                    if let Some(body) = &mut item.body {
                        assign(body, kinds, ids, next);
                    }
                }
                _ => {}
            }
        }
    }
    let mut ids = BTreeMap::new();
    reserve(stmts, kinds, &mut ids)?;
    assign(stmts, kinds, &mut ids.into_keys().collect(), &mut 1);
    Ok(())
}

fn index_nodes(
    stmts: &[Stmt],
    kinds: &BTreeSet<String>,
    nodes: &mut BTreeMap<String, Span>,
) -> Result<(), Error> {
    for stmt in stmts {
        let Stmt::Item(item) = stmt else { continue };
        if kinds.contains(&item.head) {
            let (id, span) = explicit_id(item).ok_or_else(|| {
                Error::located(
                    item.head_span,
                    "view/explicit-id",
                    "shared model nodes require explicit IDs",
                )
                .with_help("write `node stable-id \"Label\"` or use `id=stable-id`")
            })?;
            if id.is_empty() {
                return Err(Error::located(
                    span,
                    "view/empty-id",
                    "shared node IDs cannot be empty",
                ));
            }
            if let Some(first) = nodes.insert(id.clone(), span) {
                return Err(Error::located(
                    span,
                    "semantic/duplicate-id",
                    format!("duplicate node ID `{id}`"),
                )
                .with_related(first, "the first node ID is here"));
            }
        }
        if matches!(item.head.as_str(), "view" | "model" | "diagram") {
            return Err(Error::located(
                item.head_span,
                "view/nesting",
                "models and views cannot be nested inside shared diagram blocks",
            ));
        }
        if (kinds.contains(&item.head) || matches!(item.head.as_str(), "row" | "section" | "band"))
            && let Some(body) = &item.body
        {
            index_nodes(body, kinds, nodes)?;
        }
    }
    Ok(())
}

fn validate_edges(stmts: &[Stmt], nodes: &BTreeMap<String, Span>) -> Result<(), Error> {
    for stmt in stmts {
        match stmt {
            Stmt::Edge(edge) => {
                for (id, span) in [(&edge.from, edge.from_span), (&edge.to, edge.to_span)] {
                    if !nodes.contains_key(id) {
                        return Err(Error::located(
                            span,
                            "view/edge-target",
                            format!("edge refers to unknown shared node `{id}`"),
                        )
                        .with_optional_help(crate::diagnostic::suggestion(
                            id,
                            nodes.keys().map(String::as_str),
                        )));
                    }
                }
            }
            Stmt::Item(item) if !matches!(item.head.as_str(), "step" | "style" | "arrow") => {
                if let Some(body) = &item.body {
                    validate_edges(body, nodes)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

// Only valid sequence note declarations can be filtered by their targets.
// Malformed declarations stay in the selected source for semantic diagnostics.
fn note_targets(item: &Item) -> Option<Vec<(String, Span)>> {
    if item.head != "note" || item.body.is_some() {
        return None;
    }
    let mut label = false;
    let mut from = None;
    let mut to = None;
    for (arg, span) in item.args.iter().zip(&item.arg_spans) {
        match arg {
            Arg::Value(Value::Str(_)) if !label => label = true,
            Arg::Attr(key, value) if key == "over" && from.is_none() && to.is_none() => {
                from = Some((value.as_text(), *span));
                to = from.clone();
            }
            Arg::Attr(key, value) if key == "from" && from.is_none() => {
                from = Some((value.as_text(), *span))
            }
            Arg::Attr(key, value) if key == "to" && to.is_none() => {
                to = Some((value.as_text(), *span))
            }
            _ => return None,
        }
    }
    if !label || from.is_some() != to.is_some() {
        return None;
    }
    Some(from.into_iter().chain(to).collect())
}

fn validate_sequence_notes(stmts: &[Stmt], nodes: &BTreeMap<String, Span>) -> Result<(), Error> {
    for stmt in stmts {
        let Stmt::Item(item) = stmt else { continue };
        if let Some(targets) = note_targets(item) {
            for (id, span) in targets {
                if !nodes.contains_key(&id) {
                    return Err(Error::located(
                        span,
                        "sequence/note-target",
                        format!("note refers to unknown shared participant `{id}`"),
                    )
                    .with_optional_help(crate::diagnostic::suggestion(
                        &id,
                        nodes.keys().map(String::as_str),
                    )));
                }
            }
        }
        if matches!(item.head.as_str(), "loop" | "opt" | "alt" | "branch")
            && let Some(body) = &item.body
        {
            validate_sequence_notes(body, nodes)?;
        }
    }
    Ok(())
}

fn selection(view: &Item, nodes: &BTreeMap<String, Span>) -> Result<BTreeSet<String>, Error> {
    let body = view.body.as_deref().ok_or_else(|| {
        Error::located(
            view.head_span,
            "view/body",
            "a view needs a body containing include directives",
        )
    })?;
    let mut selected = BTreeSet::new();
    let mut declarations = BTreeMap::new();
    for stmt in body {
        let Stmt::Item(item) = stmt else {
            return Err(Error::located(
                stmt.span(),
                "view/directive",
                "view bodies accept only include and step directives",
            ));
        };
        if item.head == "step" {
            continue;
        }
        if item.head != "include" || item.body.is_some() || item.args.is_empty() {
            return Err(Error::located(
                item.head_span,
                "view/directive",
                "view bodies accept `include node-id ...` and step directives",
            ));
        }
        for (arg, span) in item.args.iter().zip(&item.arg_spans) {
            let (Arg::Value(Value::Ident(id)) | Arg::Value(Value::Str(id))) = arg else {
                return Err(Error::located(
                    *span,
                    "view/include-argument",
                    "include expects node IDs without attributes",
                ));
            };
            if !nodes.contains_key(id) {
                return Err(Error::located(
                    *span,
                    "view/include-target",
                    format!("unknown shared node `{id}`"),
                )
                .with_optional_help(crate::diagnostic::suggestion(
                    id,
                    nodes.keys().map(String::as_str),
                )));
            }
            if let Some(first) = declarations.insert(id.clone(), *span) {
                return Err(Error::located(
                    *span,
                    "view/duplicate-include",
                    format!("node `{id}` is included twice in this view"),
                )
                .with_related(first, "the first include is here"));
            }
            selected.insert(id.clone());
        }
    }
    if selected.is_empty() {
        return Err(Error::located(
            view.head_span,
            "view/empty",
            "a view must include at least one shared node",
        ));
    }
    Ok(selected)
}

fn filter(
    stmts: &[Stmt],
    selected: &BTreeSet<String>,
    kinds: &BTreeSet<String>,
    sequence: bool,
) -> Vec<Stmt> {
    fn expand(
        stmts: &[Stmt],
        selected: &mut BTreeSet<String>,
        kinds: &BTreeSet<String>,
        inherited: bool,
    ) -> bool {
        let mut any = false;
        for stmt in stmts {
            let Stmt::Item(item) = stmt else { continue };
            let id = if kinds.contains(&item.head) {
                explicit_id(item).map(|v| v.0)
            } else {
                None
            };
            let direct = inherited || id.as_ref().is_some_and(|id| selected.contains(id));
            let descendant = if kinds.contains(&item.head)
                || matches!(item.head.as_str(), "row" | "section" | "band")
            {
                expand(item.body.as_deref().unwrap_or(&[]), selected, kinds, direct)
            } else {
                false
            };
            if (direct || descendant)
                && let Some(id) = id
            {
                selected.insert(id);
                any = true;
            }
            any |= descendant;
        }
        any
    }
    let mut selected = selected.clone();
    expand(stmts, &mut selected, kinds, false);
    filter_inner(stmts, &selected, kinds, sequence)
}

fn filter_inner(
    stmts: &[Stmt],
    selected: &BTreeSet<String>,
    kinds: &BTreeSet<String>,
    sequence: bool,
) -> Vec<Stmt> {
    let mut out = Vec::new();
    for stmt in stmts {
        match stmt {
            Stmt::Edge(edge) if selected.contains(&edge.from) && selected.contains(&edge.to) => {
                out.push(stmt.clone())
            }
            Stmt::Edge(_) => {}
            Stmt::Item(item)
                if sequence
                    && note_targets(item).is_some_and(|targets| {
                        targets.iter().any(|(id, _)| !selected.contains(id))
                    }) => {}
            Stmt::Item(item) if kinds.contains(&item.head) => {
                if explicit_id(item).is_some_and(|(id, _)| selected.contains(&id)) {
                    let mut item = item.clone();
                    item.body = item
                        .body
                        .as_ref()
                        .map(|body| filter_inner(body, selected, kinds, sequence));
                    out.push(Stmt::Item(item));
                }
            }
            Stmt::Item(item) if matches!(item.head.as_str(), "row" | "section" | "band") => {
                let mut item = item.clone();
                let body = item.body.as_deref().unwrap_or(&[]);
                let kept = filter_inner(body, selected, kinds, sequence);
                // A wrapper without any selected node has no visual purpose.
                if !contains_node(&kept, kinds) {
                    continue;
                }
                if item.head == "row" {
                    let old_cells: Vec<_> =
                        body.iter().filter(|s| matches!(s, Stmt::Item(_))).collect();
                    let new_cells: Vec<_> =
                        kept.iter().filter(|s| matches!(s, Stmt::Item(_))).collect();
                    for arg in &mut item.args {
                        if let Arg::Weights(weights) = arg
                            && weights.len() == old_cells.len()
                        {
                            *weights = old_cells
                                .iter()
                                .zip(weights.iter())
                                .filter_map(|(old, weight)| {
                                    new_cells
                                        .iter()
                                        .any(|new| new.span() == old.span())
                                        .then_some(*weight)
                                })
                                .collect();
                        }
                    }
                }
                item.body = Some(kept);
                out.push(Stmt::Item(item));
            }
            Stmt::Item(item) if matches!(item.head.as_str(), "loop" | "opt" | "alt" | "branch") => {
                let mut item = item.clone();
                item.body = item
                    .body
                    .as_ref()
                    .map(|body| filter_inner(body, selected, kinds, sequence));
                // Empty/invalid sequence fragments still receive ordinary
                // sequence diagnostics; a view never rewrites branch logic.
                out.push(Stmt::Item(item));
            }
            Stmt::Item(item) => {
                // Unknown statements remain for ordinary semantic validation;
                // annotations on retained nodes remain attached to that node.
                out.push(Stmt::Item(item.clone()));
            }
        }
    }
    out
}

fn contains_node(stmts: &[Stmt], kinds: &BTreeSet<String>) -> bool {
    stmts.iter().any(|s| match s {
        Stmt::Item(item) if kinds.contains(&item.head) => true,
        Stmt::Item(item) if matches!(item.head.as_str(), "row" | "section" | "band") => {
            contains_node(item.body.as_deref().unwrap_or(&[]), kinds)
        }
        _ => false,
    })
}
