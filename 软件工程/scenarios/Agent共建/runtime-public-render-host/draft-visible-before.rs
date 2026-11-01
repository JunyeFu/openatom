            }
        }
    }
}

fn render_visible_member(job: &RenderJob, object: &ObjectRef) -> bool {
    let input = &job.input;
    let scenarios = input
        .scenarios
        .iter()
        .map(|s| (s.id.clone(), s.clone()))
        .collect();
    if lookup(&input.project, &scenarios, object).is_ok() {
        return true;
    }
    match object.kind {
        ObjectKind::Module => input
            .modules
            .iter()
            .any(|p| p.definition.id.as_str() == object.id.as_str()),
        ObjectKind::DefinitionDraft => {
            matches!(&job.frame.scope,EditScope::DefinitionDraft(d)if d.id.as_str()==object.id.as_str())
        }
        ObjectKind::Run => {
            input
                .run
