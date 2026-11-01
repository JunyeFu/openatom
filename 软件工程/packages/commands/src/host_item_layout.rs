//! Presentation-only declarations on existing Project owners.
use super::*;
fn layouts(project: &ProjectState) -> Vec<(ObjectRef, String, &ItemSlotLayout, bool)> {
    let mut all = vec![];
    for (index, d) in project.devices.iter().enumerate() {
        if let Some(l) = &d.item_layout {
            all.push((
                object(ObjectKind::Device, d.id.clone()),
                format!("/devices/{index}/item_layout"),
                l,
                true,
            ));
        }
    }
    for (index, d) in project.locations.iter().enumerate() {
        if let Some(l) = &d.item_layout {
            all.push((
                object(ObjectKind::Location, d.id.clone()),
                format!("/locations/{index}/item_layout"),
                l,
                false,
            ));
        }
    }
    for (index, d) in project.item_types.iter().enumerate() {
        if let Some(l) = &d.contents_layout {
            all.push((
                object(ObjectKind::ItemType, d.id.clone()),
                format!("/item_types/{index}/contents_layout"),
                l,
                true,
            ));
        }
    }
    all
}
fn object(kind: ObjectKind, id: ObjectId) -> ObjectRef {
    ObjectRef { kind, id }
}
pub(super) fn validate_references(project: &ProjectState) -> Result<(), ApiError> {
    for (_, path, layout, _) in layouts(project) {
        for (n, slot) in layout.slots.iter().enumerate() {
            if !project
                .item_types
                .iter()
                .any(|t| t.id == slot.item_type_ref.id && t.revision == slot.item_type_ref.revision)
            {
                return Err(error(
                    ErrorCode::MissingReference,
                    "Display slot must name an exact ItemType id/revision in this Project",
                    Some(&format!("{path}/slots/{n}/item_type_ref")),
                ));
            }
        }
    }
    Ok(())
}
fn resolved<'a>(
    mut p: &'a Presentation,
    definitions: &'a [ModuleDefinition],
) -> Option<&'a Presentation> {
    let mut seen = BTreeSet::new();
    loop {
        let locator = p.lod.detailed.as_ref().unwrap_or(&p.source);
        let MediaLocator::MediaRef(reference) = locator else {
            return Some(p);
        };
        if !seen.insert(module_key(reference)) {
            return None;
        }
        let definition = definitions
            .iter()
            .find(|d| d.id == reference.id && d.version == reference.version)?;
        let ModulePayload::Media(next) = &definition.payload else {
            return None;
        };
        p = next;
    }
}
pub(super) fn issues(project: &ProjectState, definitions: &[ModuleDefinition]) -> Vec<Issue> {
    let mut issues = vec![];
    for (owner, path, layout, node_required) in layouts(project) {
        let presentation = match owner.kind {
            ObjectKind::Device => {
                let d = project.devices.iter().find(|d| d.id == owner.id).unwrap();
                d.presentation_override.as_ref().or_else(|| {
                    definitions
                        .iter()
                        .find(|v| v.id == d.module_ref.id && v.version == d.module_ref.version)
                        .and_then(|v| {
                            if let ModulePayload::Devices(d) = &v.payload {
                                d.presentation.as_ref()
                            } else {
                                None
                            }
                        })
                })
            }
            ObjectKind::ItemType => project
                .item_types
                .iter()
                .find(|d| d.id == owner.id)
                .and_then(|d| d.module_ref.as_ref())
                .and_then(|r| {
                    definitions
                        .iter()
                        .find(|v| v.id == r.id && v.version == r.version)
                })
                .and_then(|v| {
                    if let ModulePayload::Items(d) = &v.payload {
                        d.presentation_or_media_ref.as_ref()
                    } else {
                        None
                    }
                }),
            _ => None,
        }
        .and_then(|p| resolved(p, definitions));
        let mut add = |field: String, message: &str| {
            issues.push(Issue {
                code: "item_layout.missing_display_source".into(),
                object_ref: Some(owner.clone()),
                field_path: Some(field),
                message: message.into(),
                blocks: vec![],
            })
        };
        if layout.slots.is_empty() {
            add(
                format!("{path}/slots"),
                "No display slots declared; business capacity is unchanged",
            );
        }
        for (n, slot) in layout.slots.iter().enumerate() {
            if node_required {
                let valid = match (presentation, slot.anchor_role.as_ref()) {
                    (Some(p), Some(role))
                        if matches!(
                            p.kind,
                            PresentationKind::Model | PresentationKind::NativeScene
                        ) =>
                    {
                        matches!(p.anchor_roles.get(role), Some(AnchorRef::SceneNode(index)) if (index.value() as usize) < p.nodes.len())
                    }
                    _ => false,
                };
                if !valid {
                    add(
                        format!("{path}/slots/{n}/anchor_role"),
                        "A scene_node anchor in the exact resolved detailed holder media is missing; no item geometry may be fabricated",
                    );
                }
            } else if slot.anchor_role.is_some() {
                add(
                    format!("{path}/slots/{n}/anchor_role"),
                    "Location display offsets are relative to the actual Location pose; no model anchor applies",
                );
            }
        }
    }
    issues
}
pub(super) fn slot_array(path: &str) -> bool {
    matches!(path, "/item_layout/slots" | "/contents_layout/slots")
}

pub(super) fn anchor_leaf(pointer: &str) -> bool {
    let fields: Vec<_> = pointer.split('/').collect();
    matches!(fields.as_slice(), ["", "item_layout" | "contents_layout", "slots", index, "anchor_role"] if index.parse::<usize>().is_ok_and(|value| value.to_string() == *index))
}
