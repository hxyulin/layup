#![allow(dead_code)]
/// Geometry assertions use authored names; opaque paths remain checked by document/API tests.
pub fn authored(id: &str) -> String {
    if let Some(path) = id
        .strip_prefix("object:")
        .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
    {
        path.last().cloned().unwrap_or_default()
    } else if let Some(id) = id
        .strip_prefix("relationship:")
        .and_then(|s| serde_json::from_str::<String>(s).ok())
    {
        id
    } else {
        id.into()
    }
}
pub fn node(scene: &layup::layout::Scene, id: &str) -> Option<usize> {
    scene
        .node(id)
        .or_else(|| scene.nodes.iter().position(|n| authored(&n.id) == id))
}
