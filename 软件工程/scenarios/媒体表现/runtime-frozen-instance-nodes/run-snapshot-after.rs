        if let ObjectSource::RunSnapshot { run_id, at_ms } = params.source {
            if matches!(
                params.object_id.kind,
                ObjectKind::Project
                    | ObjectKind::Scenario
                    | ObjectKind::Device
                    | ObjectKind::Resource
            ) {
                let summary = self.frozen_run_summary(caller, scope, &run_id, &at_ms)?;
                if matches!(
                    params.object_id.kind,
                    ObjectKind::Device | ObjectKind::Resource
                ) {
                    // Static declarations frozen in the Run input, not runtime
                    // pose/resource snapshots or the current editable Project.
                    let frozen = self.frozen_run_project(&summary)?;
                    return lookup(&frozen, &BTreeMap::new(), &params.object_id);
                }
                if matches!(params.object_id.kind, ObjectKind::Project) {
                    if params.object_id.id.as_str() != summary.input.project_id.as_str() {
                        return Err(missing(
                            &params.object_id,
                            "Project is not this Run's frozen input",
                        ));
                    }
                    return Ok(DomainObject::Project(Box::new(
                        self.frozen_run_project(&summary)?,
                    )));
                }
                if params.object_id.id.as_str() != summary.input.scenario_id.as_str() {
                    return Err(missing(
                        &params.object_id,
                        "Scenario is not this Run's frozen input",
                    ));
                }
                return Ok(DomainObject::Scenario(Box::new(
                    self.frozen_run_scenario(&summary)?,
                )));
            }
            if !caller
                .allowed_operations
                .iter()
                .any(|operation| matches!(operation, OperationName::GetRun))
            {
                return Err(error(
                    ErrorCode::PermissionDenied,
                    "Run input reads require current GetRun authority",
                    None,
                ));
            }
            let RunView::Objects(mut values) = self.get_run(
                caller,
                scope,
                GetRunParams {
                    run_id,
                    query: RunQuery::ObjectsAt {
                        at_ms,
                        objects: vec![params.object_id.clone()],
                        page: PageRequest {
                            after_index: None,
                            limit: SafeUint::try_from(1).unwrap(),
                        },
                    },
                },
            )?
            else {
                unreachable!()
            };
            return values.objects.pop().ok_or_else(|| {
                missing(
                    &params.object_id,
                    "Object does not exist at this computed Run time",
                )
            });
        }
