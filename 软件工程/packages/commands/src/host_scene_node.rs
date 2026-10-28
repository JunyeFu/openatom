//! Select only a node owned by the already-authorized exact parent object.
use super::*;

fn definition_presentation(
    definition: &ModuleDefinition,
) -> Result<Option<&Presentation>, ApiError> {
    match &definition.payload {
        ModulePayload::Media(value) => Ok(Some(value)),
        ModulePayload::Devices(value) => Ok(value.presentation.as_ref()),
        ModulePayload::Items(value) => Ok(value.presentation_or_media_ref.as_ref()),
        ModulePayload::Resources(value) => Ok(value.presentation.as_ref()),
        ModulePayload::Transport(value) => Ok(value.presentation.as_ref()),
        _ => Err(not_presentation_owner()),
    }
}

fn not_presentation_owner() -> ApiError {
    error(
        ErrorCode::InvalidInput,
        "The selected parent object does not directly own a Presentation",
        Some("/params/object_id"),
    )
}

pub(super) fn read(parent: &DomainObject, index: &SafeUint) -> Result<DomainObject, ApiError> {
    let presentation = match parent {
        DomainObject::Module(package) => definition_presentation(&package.definition)?,
        DomainObject::Definition(definition) => definition_presentation(definition)?,
        DomainObject::Device(device) => device.presentation_override.as_ref(),
        DomainObject::Resource(resource) => resource.presentation.as_ref(),
        DomainObject::NativeScene(value) => Some(value),
        _ => return Err(not_presentation_owner()),
    };
    // node_index is the canonical array position. Neither SceneNode.id nor an
    // ImportedNode geometry's GLB index selects an entry in this array. A
    // MediaRef changes no ownership here; the caller must read its exact owner.
    presentation
        .and_then(|value| {
            usize::try_from(index.value())
                .ok()
                .and_then(|i| value.nodes.get(i))
        })
        .cloned()
        .map(DomainObject::SceneNode)
        .ok_or_else(|| {
            error(
                ErrorCode::MissingReference,
                "The selected exact parent has no node at this canonical array position",
                Some("/params/node_index"),
            )
        })
}
