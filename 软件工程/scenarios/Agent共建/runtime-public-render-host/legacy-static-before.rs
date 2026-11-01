                    r.iter()
                        .filter(|r| r.0.0.value() <= at.0.value())
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            if matches!(
                params.view.kind,
                RenderViewKind::LineCanvas
                    | RenderViewKind::MultiHallCanvas
                    | RenderViewKind::DeviceDetail
                    | RenderViewKind::DeviceCyclePreview
                    | RenderViewKind::TimelineSnapshot
            ) {
                let mut selectors = vec![vec![]];
                if !run.jobs.is_empty() {
                    selectors.push(
                        run.jobs
                            .iter()
                            .map(|j| ObjectRef {
                                kind: ObjectKind::Job,
                                id: j.id.clone(),
                            })
                            .collect(),
                    );
                }
                for selected in selectors {
                    let mut after_index = None;
                    loop {
                        let RunView::Objects(page) = self.get_run(
                            caller,
                            scope,
                            GetRunParams {
                                run_id: id.clone(),
                                query: RunQuery::ObjectsAt {
                                    at_ms: at.clone(),
                                    objects: selected.clone(),
                                    page: PageRequest {
                                        after_index,
                                        limit: SafeUint::try_from(1000).unwrap(),
                                    },
                                },
                            },
                        )?
                        else {
                            unreachable!()
                        };
                        input.objects_at.extend(page.objects);
                        after_index = page.next_index;
                        if after_index.is_none() {
                            break;
                        }
                    }
                }
            }
            let reference = run.input_ref.clone();
            input.run = Some(*run);
            return Ok((
                input,
                RenderInputSource::Run {
                    input: reference,
                    definition: definition_identity,
                },
            ));
        }
        let source = match scope {
