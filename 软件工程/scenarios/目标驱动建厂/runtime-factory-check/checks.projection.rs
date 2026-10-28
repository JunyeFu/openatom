#[path = "D:/Agent/ICNSD-KEL/大作业/02_竞赛作品/超级工厂/软件工程/packages/commands/src/host_factory_check/geometry.rs"] mod geometry;
// factory-check-pure:start
pub(super) mod checks {
    //! Read-only factory diagnostics. Shared business admission is reused, not simulated.
    use super::geometry;
    use geometry::{EPS, Polygon};
    use sf_domain::*;
    use std::collections::{BTreeMap, BTreeSet};

    fn object(kind: ObjectKind, id: &str) -> ObjectRef {
        ObjectRef {
            kind,
            id: ObjectId::try_from(id.to_owned()).expect("existing typed identity"),
        }
    }
    fn same_object(a: &ObjectRef, b: &ObjectRef) -> bool {
        a.id == b.id && std::mem::discriminant(&a.kind) == std::mem::discriminant(&b.kind)
    }
    fn same_module(a: &ModuleRef, b: &ModuleRef) -> bool {
        a.id == b.id && a.version == b.version
    }
    fn exact<'a>(defs: &'a [ModuleDefinition], r: &ModuleRef) -> Option<&'a ModuleDefinition> {
        defs.iter().find(|d| d.id == r.id && d.version == r.version)
    }
    fn selected(p: &ProjectState, scope: &[ObjectRef], r: &ObjectRef) -> bool {
        let direct = scope.iter().any(|s| {
            same_object(s, r)
                || matches!(s.kind, ObjectKind::Project) && s.id.as_str() == p.id.as_str()
                || matches!(s.kind, ObjectKind::Hall)
                    && (matches!(r.kind, ObjectKind::Device)
                        && p.devices.iter().any(|d| d.id == r.id && d.hall_id == s.id)
                        || matches!(r.kind, ObjectKind::Line)
                            && p.lines.iter().any(|l| l.id == r.id && l.hall_id == s.id)
                        || matches!(r.kind, ObjectKind::Location)
                            && p.locations
                                .iter()
                                .any(|l| l.id == r.id && l.hall_id.as_ref() == Some(&s.id)))
                || matches!(s.kind, ObjectKind::Line)
                    && matches!(r.kind, ObjectKind::Device)
                    && p.devices
                        .iter()
                        .any(|d| d.id == r.id && d.line_id.as_ref() == Some(&s.id))
                || matches!(s.kind, ObjectKind::Route)
                    && matches!(r.kind, ObjectKind::Device)
                    && p.routes.iter().any(|route| {
                        route.id == s.id && route.steps.iter().any(|step| step.device_id == r.id)
                    })
        });
        if direct {
            return true;
        }
        match r.kind {
            ObjectKind::Resource => p.devices.iter().any(|d| {
                selected(p, scope, &object(ObjectKind::Device, d.id.as_str()))
                    && (d.role_bindings.resources.values().any(|id| *id == r.id)
                        || p.routes.iter().flat_map(|r| &r.steps).any(|s| {
                            s.device_id == d.id
                                && s.resource_bindings.values().any(|id| *id == r.id)
                        }))
            }),
            ObjectKind::Calendar => p.resources.iter().any(|v| {
                v.calendar_id == r.id
                    && selected(p, scope, &object(ObjectKind::Resource, v.id.as_str()))
            }),
            ObjectKind::Route => p.routes.iter().any(|route| {
                route.id == r.id
                    && route.steps.iter().any(|s| {
                        selected(p, scope, &object(ObjectKind::Device, s.device_id.as_str()))
                    })
            }),
            _ => false,
        }
    }
    fn kinds() -> [DraftCheckKind; 6] {
        use DraftCheckKind::*;
        [
            ReferencesAndUnits,
            MaterialProcessConnectivity,
            SiteBounds,
            FootprintOverlap,
            Height,
            ClearanceAndStaticLanes,
        ]
    }
    struct Report {
        checks: Vec<DraftCheck>,
        issues: Vec<Issue>,
    }
    impl Report {
        fn row(
            &mut self,
            kind: DraftCheckKind,
            outcome: CheckOutcome,
            objects: Vec<ObjectRef>,
            message: impl Into<String>,
        ) {
            self.checks.push(DraftCheck {
                kind,
                outcome,
                objects,
                message: message.into(),
            });
        }
        fn problem(
            &mut self,
            kind: DraftCheckKind,
            outcome: CheckOutcome,
            objects: Vec<ObjectRef>,
            code: &str,
            path: &str,
            message: impl Into<String>,
        ) {
            let message = message.into();
            self.issues.push(Issue {
                code: format!("factory_check.{code}"),
                object_ref: objects.first().cloned(),
                field_path: Some(path.to_owned()),
                message: message.clone(),
                blocks: vec![CapabilityPurpose::CompleteConclusion],
            });
            self.row(kind, outcome, objects, message);
        }
        fn missing(
            &mut self,
            kind: DraftCheckKind,
            r: &ObjectRef,
            code: &str,
            path: &str,
            message: impl Into<String>,
        ) {
            self.problem(
                kind,
                CheckOutcome::Undetermined,
                vec![r.clone()],
                code,
                path,
                message,
            );
        }
        fn fail(
            &mut self,
            kind: DraftCheckKind,
            r: &ObjectRef,
            code: &str,
            path: &str,
            message: impl Into<String>,
        ) {
            self.problem(
                kind,
                CheckOutcome::Fail,
                vec![r.clone()],
                code,
                path,
                message,
            );
        }
        fn pass_unless_problem(&mut self, kind: DraftCheckKind, r: &ObjectRef, message: &str) {
            if !self.checks.iter().any(|c| {
                std::mem::discriminant(&c.kind) == std::mem::discriminant(&kind)
                    && c.objects.iter().any(|o| same_object(o, r))
            }) {
                self.row(kind, CheckOutcome::Pass, vec![r.clone()], message);
            }
        }
    }

    pub(crate) fn inspect(
        p: &ProjectState,
        scenarios: &BTreeMap<ScenarioId, ScenarioInput>,
        defs: &[ModuleDefinition],
        goal: &FactoryGenerationGoal,
        scope: &[ObjectRef],
        references: bool,
        business: bool,
        layout: bool,
        profiles: &[TemplateGenerationProfile],
        profile_issues: Vec<Issue>,
    ) -> FactoryDraftCheckResult {
        let root = object(ObjectKind::Project, p.id.as_str());
        let mut out = Report {
            checks: vec![],
            issues: vec![],
        };
        if scope.is_empty() {
            for kind in kinds() {
                out.missing(
                    kind,
                    &root,
                    "empty_scope",
                    "/check_scope/objects",
                    "No objects requested; nothing was checked.",
                );
            }
        } else {
            if references {
                check_references(p, scenarios, defs, goal, scope, &mut out);
            } else {
                out.row(
                    DraftCheckKind::ReferencesAndUnits,
                    CheckOutcome::Undetermined,
                    scope.to_vec(),
                    "Reference/unit check was explicitly disabled.",
                );
            }
            if business {
                business::check_business(p, scenarios, defs, goal, scope, &mut out);
            } else {
                out.row(
                    DraftCheckKind::MaterialProcessConnectivity,
                    CheckOutcome::Undetermined,
                    scope.to_vec(),
                    "Business/connectivity check was explicitly disabled.",
                );
            }
            if layout {
                check_layout(p, goal, scope, profiles, &mut out);
            } else {
                for kind in kinds().into_iter().skip(2) {
                    out.row(
                        kind,
                        CheckOutcome::Undetermined,
                        scope.to_vec(),
                        "Static layout check was explicitly disabled.",
                    );
                }
            }
            if layout {
                for i in profile_issues {
                    out.row(
                        DraftCheckKind::ClearanceAndStaticLanes,
                        CheckOutcome::Undetermined,
                        i.object_ref.clone().into_iter().collect(),
                        i.message.clone(),
                    );
                    out.issues.push(i);
                }
            }
        }
        // No empty category is a vacuous Pass (e.g. only an unpositioned definition).
        for kind in kinds() {
            if !out
                .checks
                .iter()
                .any(|c| std::mem::discriminant(&c.kind) == std::mem::discriminant(&kind))
            {
                out.row(
                    kind,
                    CheckOutcome::Undetermined,
                    scope.to_vec(),
                    "No applicable declared subject in the requested scope.",
                );
            }
        }
        FactoryDraftCheckResult {
            project_revision: p.revision.clone(),
            scope: scope.to_vec(),
            checks: out.checks,
            issues: out.issues,
        }
    }

    fn port<'a>(
        p: &ProjectState,
        defs: &'a [ModuleDefinition],
        r: &PortRef,
    ) -> Option<&'a PortDefinition> {
        let device = p
            .devices
            .iter()
            .find(|d| d.id == r.instance_id && same_module(&d.module_ref, &r.module_ref))?;
        let ModulePayload::Devices(data) = &exact(defs, &device.module_ref)?.payload else {
            return None;
        };
        data.ports.iter().find(|port| port.id == r.port_id)
    }
    fn endpoint_valid(
        p: &ProjectState,
        defs: &[ModuleDefinition],
        ep: &MaterialEndpoint,
        source: bool,
        item: &ObjectId,
        unit: &UnitId,
    ) -> bool {
        match ep {
            MaterialEndpoint::Location(id) => p.locations.iter().any(|l| {
                l.id == *id
                    && l.accepted_item_types.contains(item)
                    && l.capacity.as_ref().is_none_or(|q| q.unit == *unit)
            }),
            MaterialEndpoint::Port(r) => port(p, defs, r).is_some_and(|p| {
                matches!(
                    (&p.direction, source),
                    (Direction::Output, true)
                        | (Direction::Input, false)
                        | (Direction::Bidirectional, _)
                ) && matches!(&p.payload,PortPayload::Material(m) if m.unit==*unit)
            }),
        }
    }
    fn endpoint_object(ep: &MaterialEndpoint) -> ObjectRef {
        match ep {
            MaterialEndpoint::Location(id) => object(ObjectKind::Location, id.as_str()),
            MaterialEndpoint::Port(r) => object(ObjectKind::Device, r.instance_id.as_str()),
        }
    }
    fn connection_id(c: &Connection) -> &ObjectId {
        match c {
            Connection::Material(c) => &c.id,
            Connection::Signal(c) => &c.id,
            Connection::Artifact(c) => &c.id,
        }
    }
    fn relevant_connection(p: &ProjectState, scope: &[ObjectRef], c: &Connection) -> bool {
        if selected(
            p,
            scope,
            &object(ObjectKind::Connection, connection_id(c).as_str()),
        ) {
            return true;
        }
        match c {
            Connection::Material(c) => {
                selected(p, scope, &endpoint_object(&c.source))
                    || selected(p, scope, &endpoint_object(&c.destination))
            }
            Connection::Signal(c) => {
                selected(
                    p,
                    scope,
                    &object(ObjectKind::Device, c.source.instance_id.as_str()),
                ) || selected(
                    p,
                    scope,
                    &object(ObjectKind::Device, c.destination.instance_id.as_str()),
                )
            }
            Connection::Artifact(c) => {
                selected(
                    p,
                    scope,
                    &object(ObjectKind::Device, c.source.instance_id.as_str()),
                ) || selected(
                    p,
                    scope,
                    &object(ObjectKind::Device, c.destination.instance_id.as_str()),
                )
            }
        }
    }
    fn target_exists(
        p: &ProjectState,
        scenarios: &BTreeMap<ScenarioId, ScenarioInput>,
        r: &ObjectRef,
    ) -> Option<bool> {
        Some(match r.kind {
            ObjectKind::Project => r.id.as_str() == p.id.as_str(),
            ObjectKind::ItemType => p.item_types.iter().any(|x| x.id == r.id),
            ObjectKind::Location => p.locations.iter().any(|x| x.id == r.id),
            ObjectKind::Hall => p.halls.iter().any(|x| x.id == r.id),
            ObjectKind::Line => p.lines.iter().any(|x| x.id == r.id),
            ObjectKind::Device => p.devices.iter().any(|x| x.id == r.id),
            ObjectKind::Resource => p.resources.iter().any(|x| x.id == r.id),
            ObjectKind::Calendar => p.calendars.iter().any(|x| x.id == r.id),
            ObjectKind::Product => p.products.iter().any(|x| x.id == r.id),
            ObjectKind::Bom => p.boms.iter().any(|x| x.id == r.id),
            ObjectKind::Route => p.routes.iter().any(|x| x.id == r.id),
            ObjectKind::Connection => p.connections.iter().any(|x| *connection_id(x) == r.id),
            ObjectKind::Module => p.module_refs.iter().any(|x| x.id.as_str() == r.id.as_str()),
            ObjectKind::Artifact => p.artifact_refs.iter().any(|x| x.as_str() == r.id.as_str()),
            ObjectKind::Scenario => scenarios.keys().any(|id| id.as_str() == r.id.as_str()),
            ObjectKind::Order => scenarios
                .values()
                .any(|s| s.orders.iter().any(|x| x.id == r.id)),
            ObjectKind::Task => scenarios
                .values()
                .any(|s| s.production_tasks.iter().any(|x| x.id == r.id)),
            ObjectKind::Supply => scenarios
                .values()
                .any(|s| s.supplies.iter().any(|x| x.id == r.id)),
            _ => return None,
        })
    }
    fn target_check(
        p: &ProjectState,
        scenarios: &BTreeMap<ScenarioId, ScenarioInput>,
        kind: DraftCheckKind,
        owner: &ObjectRef,
        target: &ObjectRef,
        path: &str,
        out: &mut Report,
    ) {
        match target_exists(p, scenarios, target) {
        Some(false) => out.fail(
            kind,
            owner,
            "target_reference",
            path,
            format!(
                "Target {:?} {} is absent from the same Project declarations.",
                target.kind,
                target.id.as_str()
            ),
        ),
        None => out.missing(
            kind,
            owner,
            "target_context_missing",
            path,
            "Target requires runtime or asset context unavailable in a static factory draft check.",
        ),
        Some(true) => {}
    }
    }
    fn connection_checks(
        p: &ProjectState,
        scenarios: &BTreeMap<ScenarioId, ScenarioInput>,
        defs: &[ModuleDefinition],
        scope: &[ObjectRef],
        kind: DraftCheckKind,
        out: &mut Report,
    ) {
        for c in p
            .connections
            .iter()
            .filter(|c| relevant_connection(p, scope, c))
        {
            let r = object(ObjectKind::Connection, connection_id(c).as_str());
            match c {
                Connection::Material(c) => {
                    if !p
                        .item_types
                        .iter()
                        .any(|i| i.id == c.item_type && i.unit == c.unit)
                    {
                        out.fail(
                            kind.clone(),
                            &r,
                            "connection_item_unit",
                            "/item_type",
                            "Connection item type or its declared unit does not match.",
                        );
                    }
                    for (ep, src, path) in [
                        (&c.source, true, "/source"),
                        (&c.destination, false, "/destination"),
                    ] {
                        if !endpoint_valid(p, defs, ep, src, &c.item_type, &c.unit) {
                            out.fail(kind.clone(),&r,"connection_endpoint",path,"Endpoint must resolve to the exact device version/material port/direction/unit or an actual compatible location.");
                        }
                    }
                    let store = p.locations.iter().find(|l| l.id == c.destination_store);
                    match store {
                    None=>out.fail(kind.clone(),&r,"receiving_store","/destination_store","Explicit receiving store does not exist; ports cannot substitute inventory."),
                    Some(store)=>{
                        if !store.accepted_item_types.contains(&c.item_type)
                            || store.capacity.as_ref().is_some_and(|q|q.unit!=c.unit) {
                            out.fail(kind.clone(),&r,"receiving_store_unit","/destination_store","Receiving store rejects the item or uses an incompatible capacity unit.");
                        }
                        if store.capacity.is_none() {
                            out.missing(kind.clone(),&r,"receiving_capacity_missing","/destination_store/capacity","Receiving capacity is missing; no unbounded cache is invented.");
                        } else if store.capacity.as_ref().is_some_and(|q|q.value.decimal_places.value()!=0) {
                            out.fail(kind.clone(),&r,"receiving_capacity_unit","/destination_store/capacity","Executable receiving capacity must be integral in the item unit.");
                        }
                        if matches!(&c.destination,MaterialEndpoint::Location(id) if *id!=store.id)
                            || matches!((&c.source,&c.destination),(MaterialEndpoint::Location(a),MaterialEndpoint::Port(_)) if *a!=store.id) {
                            out.fail(kind.clone(),&r,"receiving_store_mismatch","/destination_store","Location endpoint and receiving store disagree; no implicit transfer is invented.");
                        }
                    }
                }
                    for (q, path) in [
                        (&c.quantity_rule, "/quantity_rule"),
                        (&c.capacity_rule, "/capacity_rule"),
                    ] {
                        if q.as_ref().is_some_and(|q| q.unit != c.unit) {
                            out.fail(
                                kind.clone(),
                                &r,
                                "connection_quantity_unit",
                                path,
                                "Quantity/capacity rule has a different unit.",
                            );
                        }
                    }
                }
                Connection::Signal(c) => {
                    if let Some(target) = &c.effect_target {
                        target_check(
                            p,
                            scenarios,
                            kind.clone(),
                            &r,
                            target,
                            "/effect_target",
                            out,
                        );
                    }
                    let a = port(p, defs, &c.source);
                    let b = port(p, defs, &c.destination);
                    let valid = match (a, b) {
                        (Some(a), Some(b)) => {
                            matches!(a.direction, Direction::Output | Direction::Bidirectional)
                                && matches!(
                                    b.direction,
                                    Direction::Input | Direction::Bidirectional
                                )
                                && matches!((&a.payload,&b.payload),(PortPayload::Signal(x),PortPayload::Signal(y)) if std::mem::discriminant(&x.value_type)==std::mem::discriminant(&c.value_type)&&std::mem::discriminant(&x.value_type)==std::mem::discriminant(&y.value_type))
                        }
                        _ => false,
                    };
                    if !valid {
                        out.fail(
                            kind.clone(),
                            &r,
                            "signal_endpoint",
                            "/source",
                            "Signal endpoints/direction/value types must match exact definitions.",
                        );
                    }
                }
                Connection::Artifact(c) => {
                    if !p.bindings.iter().any(|b| b.id == c.binding_ref) {
                        out.fail(
                            kind.clone(),
                            &r,
                            "artifact_binding_reference",
                            "/binding_ref",
                            "Artifact connection requires an actual declared consumer binding.",
                        );
                    }
                    let valid = match (port(p, defs, &c.source), port(p, defs, &c.destination)) {
                        (Some(a), Some(b)) => {
                            matches!(a.direction, Direction::Output | Direction::Bidirectional)
                                && matches!(
                                    b.direction,
                                    Direction::Input | Direction::Bidirectional
                                )
                                && matches!((&a.payload,&b.payload),(PortPayload::Artifact(x),PortPayload::Artifact(y)) if x.artifact_type==y.artifact_type&&x.unit==y.unit&&std::mem::discriminant(&x.coordinate_frame)==std::mem::discriminant(&y.coordinate_frame))
                        }
                        _ => false,
                    };
                    if !valid {
                        out.fail(kind.clone(),&r,"artifact_endpoint","/source","Artifact endpoints, direction, type, units and coordinate frame must match.");
                    }
                }
            }
            out.pass_unless_problem(
            kind.clone(),
            &r,
            "Declared connection endpoints, directions, units and actual receiving store checked.",
        );
        }
    }
    fn check_references(
        p: &ProjectState,
        scenarios: &BTreeMap<ScenarioId, ScenarioInput>,
        defs: &[ModuleDefinition],
        goal: &FactoryGenerationGoal,
        scope: &[ObjectRef],
        out: &mut Report,
    ) {
        use DraftCheckKind::ReferencesAndUnits as K;
        let root = object(ObjectKind::Project, p.id.as_str());
        for i in check_business_inputs_with_definitions(p, scenarios, defs) {
            if i.object_ref.as_ref().is_some_and(|r| {
                selected(p, scope, r)
                    || matches!(r.kind, ObjectKind::Scenario)
                        && scenarios.values().any(|s| {
                            s.id.as_str() == r.id.as_str()
                                && business::scenario_applies(p, scope, s)
                        })
            }) && i.code != "business_inputs.order_due_unknown"
            {
                let outcome = if i.code == "business_inputs.missing_order_quantity"
                    || i.code == "business_inputs.operation_definition_required"
                {
                    CheckOutcome::Undetermined
                } else {
                    CheckOutcome::Fail
                };
                out.row(
                    K,
                    outcome,
                    i.object_ref.clone().into_iter().collect(),
                    i.message.clone(),
                );
                out.issues.push(i);
            }
        }
        let product = p.products.iter().find(|v| {
            v.id == goal.acceptance_scope.product_ref.id
                && v.revision == goal.acceptance_scope.product_ref.revision
        });
        if product.is_none() {
            out.fail(
                K,
                &root,
                "goal_product",
                "/generation_goal/acceptance_scope/product_ref",
                "Acceptance product revision does not exist.",
            );
        }
        if let Some(product) = product {
            if !p.item_types.iter().any(|i| {
                i.id == product.item_type_ref.id
                    && i.revision == product.item_type_ref.revision
                    && i.unit == goal.acceptance_scope.unit
            }) {
                out.fail(K,&root,"goal_unit","/generation_goal/acceptance_scope/unit","Acceptance unit must match the exact product item revision; no conversion is guessed.");
            }
        }
        if !p
            .locations
            .iter()
            .any(|l| l.id == goal.acceptance_scope.destination)
        {
            out.fail(
                K,
                &root,
                "goal_destination",
                "/generation_goal/acceptance_scope/destination",
                "Acceptance destination is not an actual project location.",
            );
        }
        if goal
            .accepted_quantity_per_day
            .as_ref()
            .is_some_and(|q| q.unit != goal.acceptance_scope.unit)
        {
            out.fail(
                K,
                &root,
                "goal_quantity_unit",
                "/generation_goal/accepted_quantity_per_day/unit",
                "Demand and acceptance units differ.",
            );
        }
        if let Some(r) = &goal.product_specification_ref {
            if !p
                .products
                .iter()
                .any(|v| v.id == r.id && v.revision == r.revision)
            {
                out.fail(
                    K,
                    &root,
                    "product_specification",
                    "/generation_goal/product_specification_ref",
                    "Explicit product specification revision does not exist.",
                );
            }
        }
        for r in &goal.existing_equipment {
            if !matches!(r.kind, ObjectKind::Device) || !p.devices.iter().any(|d| d.id == r.id) {
                out.fail(
                    K,
                    &root,
                    "existing_equipment",
                    "/generation_goal/existing_equipment",
                    "Declared existing equipment is not an actual Device.",
                );
            }
        }
        for device in &p.devices {
            let r = object(ObjectKind::Device, device.id.as_str());
            if !selected(p, scope, &r) {
                continue;
            }
            if !p.halls.iter().any(|h| h.id == device.hall_id) {
                out.fail(
                    K,
                    &r,
                    "device_hall",
                    "/hall_id",
                    "Device hall reference does not exist.",
                );
            }
            if let Some(id) = &device.line_id {
                if !p.lines.iter().any(|l| {
                    l.id == *id && l.hall_id == device.hall_id && l.members.contains(&device.id)
                }) {
                    out.fail(
                        K,
                        &r,
                        "device_line",
                        "/line_id",
                        "Line parent, hall and membership must agree.",
                    );
                }
            }
            if !matches!(
                exact(defs, &device.module_ref).map(|d| &d.payload),
                Some(ModulePayload::Devices(_))
            ) {
                out.fail(
                    K,
                    &r,
                    "device_module",
                    "/module_ref",
                    "Exact device module version is absent or has a different category.",
                );
            }
            for reference in device
                .operation_binding
                .iter()
                .chain(device.route_operation_refs.iter())
            {
                if !matches!(
                    exact(defs, reference).map(|d| &d.payload),
                    Some(ModulePayload::Operations(_))
                ) {
                    out.fail(
                        K,
                        &r,
                        "operation_module",
                        "/operation_binding",
                        "Referenced exact operation module does not exist.",
                    );
                }
            }
            for id in device.role_bindings.item_types.values() {
                if !p.item_types.iter().any(|i| i.id == *id) {
                    out.fail(
                        K,
                        &r,
                        "bound_item",
                        "/role_bindings/item_types",
                        "Bound item type does not exist.",
                    );
                }
            }
            for id in device.role_bindings.resources.values() {
                if !p.resources.iter().any(|i| i.id == *id) {
                    out.fail(
                        K,
                        &r,
                        "bound_resource",
                        "/role_bindings/resources",
                        "Bound resource does not exist.",
                    );
                }
            }
            for target in device.role_bindings.targets.values() {
                target_check(p, scenarios, K, &r, target, "/role_bindings/targets", out);
            }
            for reference in device.role_bindings.ports.values() {
                if port(p, defs, reference).is_none() {
                    out.fail(
                        K,
                        &r,
                        "bound_port_reference",
                        "/role_bindings/ports",
                        "Bound port must exist on the exact referenced device module and instance.",
                    );
                }
            }
            out.pass_unless_problem(
                K,
                &r,
                "Exact module, hall/line hierarchy and declared role references checked.",
            );
        }
        for hall in &p.halls {
            let r = object(ObjectKind::Hall, hall.id.as_str());
            if !selected(p, scope, &r) {
                continue;
            }
            for id in hall.input_locations.iter().chain(&hall.output_locations) {
                if !p.locations.iter().any(|l| l.id == *id) {
                    out.fail(
                        K,
                        &r,
                        "hall_location_reference",
                        "/input_locations",
                        "Hall input/output location does not exist.",
                    );
                }
            }
            out.pass_unless_problem(K, &r, "Declared hall input/output locations checked.");
        }
        for location in &p.locations {
            let r = object(ObjectKind::Location, location.id.as_str());
            if !selected(p, scope, &r) {
                continue;
            }
            if location
                .hall_id
                .as_ref()
                .is_some_and(|id| !p.halls.iter().any(|h| h.id == *id))
            {
                out.fail(
                    K,
                    &r,
                    "location_hall_reference",
                    "/hall_id",
                    "Location parent hall is absent.",
                );
            }
            for id in &location.accepted_item_types {
                if !p.item_types.iter().any(|i| i.id == *id) {
                    out.fail(
                        K,
                        &r,
                        "location_item_reference",
                        "/accepted_item_types",
                        "Accepted item type does not exist.",
                    );
                }
            }
            out.pass_unless_problem(
                K,
                &r,
                "Declared location parent and accepted item types checked.",
            );
        }
        for line in &p.lines {
            let r = object(ObjectKind::Line, line.id.as_str());
            if !selected(p, scope, &r) {
                continue;
            }
            if !p.halls.iter().any(|h| h.id == line.hall_id) {
                out.fail(K, &r, "line_hall", "/hall_id", "Line hall is absent.");
            }
            for id in &line.members {
                if !p.devices.iter().any(|d| {
                    d.id == *id && d.hall_id == line.hall_id && d.line_id.as_ref() == Some(&line.id)
                }) {
                    out.fail(
                        K,
                        &r,
                        "line_member",
                        "/members",
                        "Every line member must be a device with this line and hall parent.",
                    );
                }
            }
            for reference in &line.boundary_ports {
                if port(p, defs, reference).is_none() {
                    out.fail(
                        K,
                        &r,
                        "line_boundary_port",
                        "/boundary_ports",
                        "Line boundary port does not resolve to its exact device/version/port.",
                    );
                }
            }
            out.pass_unless_problem(K, &r, "Line hall and actual members checked.");
        }
        for resource in &p.resources {
            let r = object(ObjectKind::Resource, resource.id.as_str());
            if !selected(p, scope, &r) {
                continue;
            }
            if !matches!(
                exact(defs, &resource.definition_ref).map(|d| &d.payload),
                Some(ModulePayload::Resources(_))
            ) {
                out.fail(
                    K,
                    &r,
                    "resource_module",
                    "/definition_ref",
                    "Exact resource definition is absent or wrong category.",
                );
            }
            if !p.calendars.iter().any(|c| c.id == resource.calendar_id) {
                out.missing(
                    K,
                    &r,
                    "resource_calendar",
                    "/calendar_id",
                    "Resource shift calendar is missing; no all-day calendar is supplied.",
                );
            }
            if resource.capacity.value.decimal_places.value() != 0 {
                out.fail(
                    K,
                    &r,
                    "resource_capacity",
                    "/capacity",
                    "Resource slot capacity must be integral.",
                );
            }
            out.pass_unless_problem(
                K,
                &r,
                "Resource definition, capacity and explicit calendar checked.",
            );
        }
        if selected(p, scope, &root) {
            for reference in &p.module_refs {
                if exact(defs, reference).is_none() {
                    out.fail(
                        K,
                        &root,
                        "project_module",
                        "/module_refs",
                        format!(
                            "Exact module {}@{} is not loaded.",
                            reference.id.as_str(),
                            String::from(reference.version.clone())
                        ),
                    );
                }
            }
        }
        connection_checks(p, scenarios, defs, scope, K, out);
        out.pass_unless_problem(K,&root,"Explicit generation goal product, units and destination checked; no production result inferred.");
    }

    fn unknown_issue(i: &Issue) -> bool {
        matches!(
            i.code.as_str(),
            "business_inputs.missing_order_quantity"
                | "business_inputs.operation_definition_required"
                | "business_inputs.order_due_unknown"
        )
    }
    fn duration_check(value: Option<&DurationValue>, r: &ObjectRef, path: &str, out: &mut Report) {
        if value.is_none_or(|v| matches!(v.basis, SourceBasis::Unknown)) {
            out.missing(
            DraftCheckKind::MaterialProcessConnectivity,
            r,
            "duration_missing",
            path,
            "Execution duration is missing or its source is unknown; no cycle time is invented.",
        );
        }
    }
    struct Placed {
        poly: Polygon,
        z: f64,
    }
    fn placement(p: &ProjectState, d: &DeviceInstance) -> Result<Placed, (&'static str, bool)> {
        if d.footprint_m.is_empty() {
            return Err(("physical footprint_m is missing", true));
        }
        let local = geometry::polygon(&d.footprint_m).map_err(|e| (e, false))?;
        let hall = p
            .halls
            .iter()
            .find(|h| h.id == d.hall_id)
            .ok_or(("hall parent is absent", false))?;
        let mut poses = vec![&hall.origin];
        if let Some(id) = &d.line_id {
            let line = p
                .lines
                .iter()
                .find(|l| l.id == *id && l.hall_id == d.hall_id)
                .ok_or(("line parent is absent or in a different hall", false))?;
            poses.push(&line.transform);
        }
        poses.push(&d.pose);
        let (poly, z) = geometry::transform(&local, &poses)
            .map_err(|e| (e, e.starts_with("tilted") || e.starts_with("unsupported")))?;
        Ok(Placed { poly, z })
    }
    fn check_layout(
        p: &ProjectState,
        goal: &FactoryGenerationGoal,
        scope: &[ObjectRef],
        profiles: &[TemplateGenerationProfile],
        out: &mut Report,
    ) {
        use DraftCheckKind::*;
        let site = &goal.site_envelope;
        let root = object(ObjectKind::Project, p.id.as_str());
        let mut boundaries = vec![];
        let mut site_invalid = false;
        match (&site.length_m, &site.width_m) {
            (Some(w), Some(d)) => match geometry::rectangle(w.value(), d.value()) {
                Ok(poly) => boundaries.push(poly),
                Err(e) => {
                    site_invalid = true;
                    out.fail(
                        SiteBounds,
                        &root,
                        "site_dimensions",
                        "/generation_goal/site_envelope",
                        e,
                    );
                }
            },
            (None, None) => {}
            _ => {
                site_invalid = true;
                out.missing(
                SiteBounds,
                &root,
                "site_dimensions",
                "/generation_goal/site_envelope",
                "Both site length and width are required when a rectangular envelope is supplied.",
            );
            }
        }
        if !site.usable_polygon_m.is_empty() {
            match geometry::polygon(&site.usable_polygon_m) {
                Ok(poly) => boundaries.push(poly),
                Err(e) => {
                    site_invalid = true;
                    out.fail(
                        SiteBounds,
                        &root,
                        "site_polygon",
                        "/generation_goal/site_envelope/usable_polygon_m",
                        e,
                    );
                }
            }
        }
        let mut regions = vec![];
        let mut region_invalid = false;
        for reg in site.regions.iter().chain(
            profiles
                .iter()
                .flat_map(|p| p.site_applicability.regions.iter()),
        ) {
            match geometry::polygon(&reg.polygon_m) {
                Ok(poly) => regions.push((reg, poly)),
                Err(e) => {
                    region_invalid = true;
                    out.fail(
                        ClearanceAndStaticLanes,
                        &root,
                        "region_polygon",
                        "/generation_goal/site_envelope/regions",
                        format!("Region {}: {e}", reg.id.as_str()),
                    );
                }
            }
        }
        let usable: Vec<_> = regions
            .iter()
            .filter(|(r, _)| matches!(r.kind, LayoutRegionKind::Usable))
            .map(|(_, p)| p.clone())
            .collect();
        let mut clearances = vec![];
        for v in site.clearance_m.iter().chain(
            profiles
                .iter()
                .filter_map(|p| p.site_applicability.clearance_m.as_ref()),
        ) {
            if v.value() < 0. {
                out.fail(
                    ClearanceAndStaticLanes,
                    &root,
                    "clearance_negative",
                    "/generation_goal/site_envelope/clearance_m",
                    "Declared static clearance must be nonnegative.",
                );
                region_invalid = true;
            } else {
                clearances.push(v.value());
            }
        }
        let clearance = clearances.into_iter().reduce(f64::max);
        let placements: Vec<_> = p.devices.iter().map(|d| (d, placement(p, d))).collect();
        let chosen: Vec<_> = placements
            .iter()
            .filter(|(d, _)| selected(p, scope, &object(ObjectKind::Device, d.id.as_str())))
            .collect();
        if chosen.is_empty() {
            for k in [
                SiteBounds,
                FootprintOverlap,
                Height,
                ClearanceAndStaticLanes,
            ] {
                out.missing(
                    k,
                    &root,
                    "spatial_subject_missing",
                    "/check_scope/objects",
                    "No actual device footprint selected for static factory placement checks.",
                );
            }
            return;
        }
        for (d, result) in &chosen {
            let r = object(ObjectKind::Device, d.id.as_str());
            let placed = match result {
                Ok(v) => v,
                Err((message, unknown)) => {
                    for k in [
                        SiteBounds,
                        FootprintOverlap,
                        Height,
                        ClearanceAndStaticLanes,
                    ] {
                        out.problem(
                            k,
                            if *unknown {
                                CheckOutcome::Undetermined
                            } else {
                                CheckOutcome::Fail
                            },
                            vec![r.clone()],
                            "placement",
                            "/footprint_m",
                            *message,
                        );
                    }
                    continue;
                }
            };
            let hall = p.halls.iter().find(|h| h.id == d.hall_id).unwrap();
            let hall_poly = geometry::rectangle(hall.width_m.value(), hall.depth_m.value())
                .and_then(|poly| geometry::transform(&poly, &[&hall.origin]).map(|v| v.0));
            match hall_poly {
                Ok(poly) => {
                    if !geometry::contained(&placed.poly, &[poly]) {
                        out.fail(SiteBounds,&r,"hall_bounds","/pose","Declared physical footprint extends outside its actual transformed hall boundary.");
                    }
                }
                Err(e) => out.fail(SiteBounds, &r, "hall_dimensions", "/hall_id", e),
            }
            if boundaries.is_empty() && usable.is_empty() {
                out.missing(
                    SiteBounds,
                    &r,
                    "site_missing",
                    "/generation_goal/site_envelope",
                    "Actual site dimensions or usable polygon are missing.",
                );
            }
            for boundary in &boundaries {
                if !geometry::contained(&placed.poly, std::slice::from_ref(boundary)) {
                    out.fail(SiteBounds,&r,"site_bounds","/pose","Physical footprint crosses the declared site boundary (edge/interior checked, not just vertices).");
                }
            }
            if !usable.is_empty() && !geometry::contained(&placed.poly, &usable) {
                out.fail(
                    SiteBounds,
                    &r,
                    "usable_region",
                    "/pose",
                    "Physical footprint leaves the declared usable regions.",
                );
            }
            if site_invalid {
                out.missing(SiteBounds,&r,"site_incomplete","/generation_goal/site_envelope","Site definition contains invalid or incomplete geometry; no complete placement Pass.");
            }
            if let Some(id) = &d.line_id {
                let line = p.lines.iter().find(|l| l.id == *id).unwrap();
                if !line.region_polygon_m.is_empty() {
                    match geometry::polygon(&line.region_polygon_m).and_then(|poly| {
                        geometry::transform(&poly, &[&hall.origin, &line.transform]).map(|v| v.0)
                    }) {
                        Ok(poly) => {
                            if !geometry::contained(&placed.poly, &[poly]) {
                                out.fail(
                                    SiteBounds,
                                    &r,
                                    "line_region",
                                    "/line_id",
                                    "Device physical footprint leaves its transformed line region.",
                                );
                            }
                        }
                        Err(e) => out.fail(SiteBounds, &r, "line_region_invalid", "/line_id", e),
                    }
                }
            }
            out.pass_unless_problem(SiteBounds,&r,"Declared metre footprint checked in project coordinates through Hall → Line → Device rigid transforms; visual scale excluded.");
            // Check selected devices against all actual same-hall obstacles, including
            // unselected devices. Selection is not permission to overlook an obstacle.
            for (other, other_placed) in &placements {
                if other.id == d.id || other.hall_id != d.hall_id {
                    continue;
                }
                let pair = vec![r.clone(), object(ObjectKind::Device, other.id.as_str())];
                match other_placed {
                    Err((message, _)) => {
                        out.problem(
                            FootprintOverlap,
                            CheckOutcome::Undetermined,
                            pair.clone(),
                            "obstacle_unknown",
                            "/footprint_m",
                            format!(
                                "Cannot exclude conflict with {}: {message}",
                                other.id.as_str()
                            ),
                        );
                        out.problem(
                            ClearanceAndStaticLanes,
                            CheckOutcome::Undetermined,
                            pair,
                            "obstacle_unknown",
                            "/footprint_m",
                            format!(
                                "Cannot establish separation from {}: {message}",
                                other.id.as_str()
                            ),
                        );
                    }
                    Ok(other_placed) => {
                        if geometry::overlap(&placed.poly, &other_placed.poly) {
                            out.problem(FootprintOverlap,CheckOutcome::Fail,pair.clone(),"footprint_overlap","/footprint_m","Declared physical polygons have positive-area overlap; visual scale and LOD do not change it.");
                        }
                        if let Some(gap) = clearance {
                            if geometry::distance(&placed.poly, &other_placed.poly) + EPS < gap {
                                out.problem(
                                    ClearanceAndStaticLanes,
                                    CheckOutcome::Fail,
                                    pair,
                                    "device_clearance",
                                    "/generation_goal/site_envelope/clearance_m",
                                    format!(
                                        "Actual polygon distance to {} is below declared {gap} m.",
                                        other.id.as_str()
                                    ),
                                );
                            }
                        }
                    }
                }
            }
            out.pass_unless_problem(FootprintOverlap,&r,"Actual same-hall polygons checked for positive-area intersections, including edge crossings and concave geometry.");
            match &d.height_m {
                None => out.missing(
                    Height,
                    &r,
                    "device_height_missing",
                    "/height_m",
                    "Device physical height is missing; visual geometry is not a height source.",
                ),
                Some(h) if h.value() <= 0. => out.fail(
                    Height,
                    &r,
                    "device_height_invalid",
                    "/height_m",
                    "Physical device height must be positive.",
                ),
                Some(h) => {
                    let bottom = placed.z;
                    let top = bottom + h.value();
                    let floor = hall.origin.translation.z.value();
                    if bottom + EPS < floor {
                        out.fail(
                            Height,
                            &r,
                            "below_floor",
                            "/pose/translation/z",
                            "Device is placed below its actual hall floor.",
                        );
                    }
                    match &hall.height_m {
                        None => out.missing(
                            Height,
                            &r,
                            "hall_height_missing",
                            "/hall_id/height_m",
                            "Hall height is missing.",
                        ),
                        Some(height) if height.value() <= 0. => out.fail(
                            Height,
                            &r,
                            "hall_height_invalid",
                            "/hall_id/height_m",
                            "Hall height must be positive.",
                        ),
                        Some(height) => {
                            if top > floor + height.value() + EPS {
                                out.fail(
                                    Height,
                                    &r,
                                    "hall_height",
                                    "/height_m",
                                    "Device top exceeds the actual hall ceiling.",
                                );
                            }
                        }
                    }
                    match &site.height_m {
                    None => out.missing(
                        Height,
                        &r,
                        "site_height_missing",
                        "/generation_goal/site_envelope/height_m",
                        "Actual site height is missing; no clearance is inferred from the picture.",
                    ),
                    Some(height) if height.value() <= 0. => out.fail(
                        Height,
                        &r,
                        "site_height_invalid",
                        "/generation_goal/site_envelope/height_m",
                        "Site height must be positive.",
                    ),
                    Some(height) => {
                        if top > height.value() + EPS {
                            out.fail(
                                Height,
                                &r,
                                "site_height",
                                "/height_m",
                                "Device top exceeds the declared project-coordinate site ceiling.",
                            );
                        }
                    }
                }
                }
            }
            out.pass_unless_problem(Height,&r,"Declared physical height, floor elevation, hall and site ceilings checked; not dynamic clearance or safety certification.");
            if regions.is_empty() {
                out.missing(ClearanceAndStaticLanes,&r,"static_regions_missing","/generation_goal/site_envelope/regions","No usable/reserved/static-lane region declaration was supplied. An empty region list cannot certify static lanes.");
            }
            if clearance.is_none() {
                out.missing(
                    ClearanceAndStaticLanes,
                    &r,
                    "clearance_missing",
                    "/generation_goal/site_envelope/clearance_m",
                    "No explicit site/template minimum spacing was declared; zero is not assumed.",
                );
            }
            if region_invalid {
                out.missing(ClearanceAndStaticLanes,&r,"region_incomplete","/generation_goal/site_envelope/regions","A declared region or clearance is invalid; complete static-lane checking is blocked.");
            }
            for (reg, poly) in &regions {
                if matches!(reg.kind, LayoutRegionKind::Usable) {
                    continue;
                }
                if reg.clearance_m.as_ref().is_some_and(|g| g.value() < 0.) {
                    out.fail(
                        ClearanceAndStaticLanes,
                        &r,
                        "region_clearance",
                        "/generation_goal/site_envelope/regions",
                        format!("Region {} has negative clearance.", reg.id.as_str()),
                    );
                    continue;
                }
                if geometry::overlap(&placed.poly, poly)
                    || reg
                        .clearance_m
                        .as_ref()
                        .is_some_and(|g| geometry::distance(&placed.poly, poly) + EPS < g.value())
                {
                    out.fail(ClearanceAndStaticLanes,&r,"reserved_region","/pose",format!("Physical footprint conflicts with reserved {:?} {} or its explicit margin.",reg.kind,reg.id.as_str()));
                }
            }
            out.pass_unless_problem(ClearanceAndStaticLanes,&r,"Declared site/template spacing and explicit keep-out/static-lane polygons checked; no undeclared or dynamic lane is certified.");
        }
    }

    mod business {
        //! Declaration-level process checks: complexity depends on the engineering
        //! graph, never on the number of demanded pieces. Does not construct an Engine.
        use super::*;
        use DraftCheckKind::MaterialProcessConnectivity as K;

        fn operation<'a>(
            defs: &'a [ModuleDefinition],
            d: &DeviceInstance,
            reference: Option<&ModuleRef>,
        ) -> Option<&'a OperationDefinition> {
            let r = reference
                .or(d.operation_binding.as_ref())
                .unwrap_or(&d.module_ref);
            match &exact(defs, r)?.payload {
                ModulePayload::Operations(op) => Some(op),
                ModulePayload::Devices(data) => match data.behavior.selected {
                    Some(BehaviorSourceKind::Inline) => data.behavior.inline.as_ref(),
                    Some(BehaviorSourceKind::OperationRef) => data
                        .behavior
                        .operation_ref
                        .as_ref()
                        .and_then(|r| exact(defs, r))
                        .and_then(|d| {
                            if let ModulePayload::Operations(op) = &d.payload {
                                Some(op)
                            } else {
                                None
                            }
                        }),
                    None => None,
                },
                _ => None,
            }
        }
        fn calendar(p: &ProjectState, id: &ObjectId, r: &ObjectRef, path: &str, out: &mut Report) {
            match p.calendars.iter().find(|c| c.id == *id) {
                None => out.missing(
                    K,
                    r,
                    "calendar_missing",
                    path,
                    format!(
                        "Explicit shift calendar {} is missing; no all-day default.",
                        id.as_str()
                    ),
                ),
                Some(c) => {
                    if matches!(c.source, SourceBasis::Unknown) {
                        out.missing(
                            K,
                            r,
                            "calendar_source",
                            path,
                            "Shift calendar has unknown source.",
                        );
                    }
                    if c.intervals
                        .iter()
                        .any(|i| i.end_ms.0.value() < i.start_ms.0.value())
                    {
                        out.fail(
                            K,
                            r,
                            "calendar_interval",
                            path,
                            "Calendar interval ends before it starts.",
                        );
                    }
                }
            }
        }
        fn scaled(q: &Quantity) -> u128 {
            u128::from(q.value.coefficient.value())
                * 10_u128.pow(9 - u32::from(q.value.decimal_places.value()))
        }
        fn resources(
            p: &ProjectState,
            d: &DeviceInstance,
            step: Option<&RouteStep>,
            requirements: &[&ResourceRequirement],
            out: &mut Report,
        ) {
            let r = object(ObjectKind::Device, d.id.as_str());
            let mut totals = BTreeMap::<ObjectId, (u128, UnitId)>::new();
            for req in requirements {
                let binding = step
                    .and_then(|s| s.resource_bindings.get(&req.role))
                    .or_else(|| d.role_bindings.resources.get(&req.role));
                let Some(id) = binding else {
                    out.missing(
                        K,
                        &r,
                        "resource_role_missing",
                        "/role_bindings/resources",
                        format!("Resource role {} is unbound.", req.role.as_str()),
                    );
                    continue;
                };
                let Some(resource) = p.resources.iter().find(|v| v.id == *id) else {
                    out.fail(
                        K,
                        &r,
                        "resource_reference",
                        "/role_bindings/resources",
                        format!("Resource {} does not exist.", id.as_str()),
                    );
                    continue;
                };
                if matches!(resource.kind, ResourceKind::ServiceDeclaration) {
                    out.missing(K,&r,"resource_solver_deferred","/role_bindings/resources","Service declaration has no implemented execution solver; it is not a worker, fixture or carrier.");
                }
                if resource.capacity.unit != req.quantity.unit
                    || req.quantity.value.decimal_places.value() != 0
                    || req.quantity.value.coefficient.value() == 0
                    || resource.capacity.value.decimal_places.value() != 0
                {
                    out.fail(
                K,
                &r,
                "resource_unit",
                "/role_bindings/resources",
                "Declared resource requirements/capacity must use consistent integral units.",
            );
                    continue;
                }
                if !req
                    .qualifications
                    .iter()
                    .all(|q| resource.qualifications.contains(q))
                {
                    out.fail(
                        K,
                        &r,
                        "resource_qualification",
                        "/role_bindings/resources",
                        "Bound resource lacks a required qualification.",
                    );
                }
                calendar(
                    p,
                    &resource.calendar_id,
                    &r,
                    "/role_bindings/resources/calendar_id",
                    out,
                );
                let total = totals
                    .entry(resource.id.clone())
                    .or_insert((0, req.quantity.unit.clone()));
                total.0 = total.0.max(scaled(&req.quantity));
            }
            for (id, (needed, _)) in totals {
                let resource = p.resources.iter().find(|r| r.id == id).unwrap();
                if needed > scaled(&resource.capacity) {
                    out.fail(
                K,
                &r,
                "resource_capacity",
                "/role_bindings/resources",
                "Required claim quantity exceeds the bound pool; repeated job/phase roles use the shared Engine maximum-per-pool rule.",
            );
                }
            }
        }
        fn material(
            p: &ProjectState,
            defs: &[ModuleDefinition],
            d: &DeviceInstance,
            step: Option<&RouteStep>,
            row: &MaterialRoleQuantity,
            output: bool,
            out: &mut Report,
        ) {
            let r = object(ObjectKind::Device, d.id.as_str());
            let path = format!(
                "/{}/{}/{}",
                if output { "outputs" } else { "inputs" },
                row.role.as_str(),
                row.port.as_str()
            );
            let binding = step
                .and_then(|s| s.item_bindings.get(&row.role))
                .or_else(|| d.role_bindings.item_types.get(&row.role));
            let Some(id) = binding else {
                out.missing(
                    K,
                    &r,
                    "material_role_missing",
                    &path,
                    "Required material role is unbound.",
                );
                return;
            };
            let Some(item) = p.item_types.iter().find(|i| i.id == *id) else {
                out.fail(
                    K,
                    &r,
                    "material_reference",
                    &path,
                    "Bound material item type does not exist.",
                );
                return;
            };
            if item.unit != row.quantity.unit
                || row.quantity.value.coefficient.value() == 0
                || row.quantity.value.decimal_places.value() != 0
            {
                out.fail(K,&r,"material_quantity",&path,"Executable discrete material quantity must be a positive integer in the bound item unit.");
            }
            let reference = PortRef {
                module_ref: d.module_ref.clone(),
                instance_id: d.id.clone(),
                port_id: row.port.clone(),
            };
            if !port(p,defs,&reference).is_some_and(|port|matches!((&port.direction,output),(Direction::Output,true)|(Direction::Input,false)|(Direction::Bidirectional,_))&&matches!(&port.payload,PortPayload::Material(m) if m.item_role==row.role&&m.unit==row.quantity.unit)) {
        out.fail(K,&r,"operation_port",&path,"Operation material role/port/direction/unit disagrees with the exact device declaration.");
    }
            let matches: Vec<_> = p
                .connections
                .iter()
                .filter_map(|c| match c {
                    Connection::Material(c)
                        if c.item_type == *id
                            && c.unit == item.unit
                            && match if output { &c.source } else { &c.destination } {
                                MaterialEndpoint::Port(r) => {
                                    r.instance_id == d.id
                                        && r.port_id == row.port
                                        && same_module(&r.module_ref, &d.module_ref)
                                }
                                _ => false,
                            } =>
                    {
                        Some(c)
                    }
                    _ => None,
                })
                .collect();
            match matches.len(){0=>out.missing(K,&r,"material_connection_missing",&path,"Exactly one declared material connection and receiving store are required; no hidden port stock is supplied."),1=>{},_=>out.fail(K,&r,"material_connection_ambiguous",&path,"Multiple connections compete for one declared execution material endpoint.")}
        }
        fn operation_checks(
            p: &ProjectState,
            defs: &[ModuleDefinition],
            d: &DeviceInstance,
            step: Option<&RouteStep>,
            op: &OperationDefinition,
            out: &mut Report,
        ) {
            let r = object(ObjectKind::Device, d.id.as_str());
            let reference = step
                .map(|s| &s.operation_ref)
                .or(d.operation_binding.as_ref())
                .unwrap_or(&d.module_ref);
            let effective = super::parameters::effective(p, defs, d, reference, op, out);
            let op = &effective;
            if op.inputs.is_empty() || op.outputs.is_empty() {
                out.missing(K,&r,"material_graph_missing","/behavior/inputs","Selected production rule lacks declared input/output material rows; an empty material graph cannot prove connectivity.");
            }

            if op.phases.is_empty() {
                duration_check(op.duration.as_ref(), &r, "/behavior/duration", out);
                resources(
                    p,
                    d,
                    step,
                    &op.resource_roles.iter().collect::<Vec<_>>(),
                    out,
                );
            } else {
                if op.duration.is_some() {
                    out.fail(
                        K,
                        &r,
                        "duplicate_duration",
                        "/behavior/duration",
                        "Phased behavior must not add another whole-cycle duration.",
                    );
                }
                for phase in &op.phases {
                    duration_check(
                        phase.duration.as_ref(),
                        &r,
                        &format!("/behavior/phases/{}/duration", phase.id.as_str()),
                        out,
                    );
                    resources(
                        p,
                        d,
                        step,
                        &op.resource_roles
                            .iter()
                            .chain(&phase.phase_resources)
                            .collect::<Vec<_>>(),
                        out,
                    );
                }
            }
            for row in &op.inputs {
                material(p, defs, d, step, row, false, out);
            }
            for row in &op.outputs {
                material(p, defs, d, step, row, true, out);
            }
            let capacity = step
                .and_then(|s| {
                    s.resource_bindings
                        .iter()
                        .find(|(role, _)| role.as_str() == "execution_capacity")
                        .map(|(_, id)| id)
                })
                .or_else(|| {
                    d.role_bindings
                        .resources
                        .iter()
                        .find(|(role, _)| role.as_str() == "execution_capacity")
                        .map(|(_, id)| id)
                });
            match capacity.and_then(|id| p.resources.iter().find(|v| v.id == *id)) {
                None => out.missing(
                    K,
                    &r,
                    "execution_capacity_missing",
                    "/role_bindings/resources/execution_capacity",
                    "Station needs an explicitly bound fixture capacity and shift calendar.",
                ),
                Some(c) => {
                    if !matches!(c.kind, ResourceKind::Fixture)
                        || c.capacity.value.coefficient.value() == 0
                        || c.capacity.value.decimal_places.value() != 0
                    {
                        out.fail(
                            K,
                            &r,
                            "execution_capacity",
                            "/role_bindings/resources/execution_capacity",
                            "Station execution capacity must be a positive integral fixture pool.",
                        );
                    }
                    calendar(
                        p,
                        &c.calendar_id,
                        &r,
                        "/role_bindings/resources/execution_capacity/calendar_id",
                        out,
                    );
                }
            }
        }
        fn linked(
            start: &ObjectId,
            end: &ObjectId,
            item: &ObjectId,
            scenarios: &BTreeMap<ScenarioId, ScenarioInput>,
        ) -> bool {
            if start == end {
                return true;
            }
            !scenarios.is_empty()
                && scenarios.values().all(|s| {
                    let mut seen = BTreeSet::from([start.clone()]);
                    loop {
                        let old = seen.len();
                        for supply in &s.supplies {
                            if supply.item_type_ref.id == *item && seen.contains(&supply.source) {
                                seen.insert(supply.destination.clone());
                            }
                        }
                        if seen.contains(end) {
                            return true;
                        }
                        if old == seen.len() {
                            return false;
                        }
                    }
                })
        }
        fn route_scenarios(
            route: &RouteDefinition,
            scenarios: &BTreeMap<ScenarioId, ScenarioInput>,
        ) -> BTreeMap<ScenarioId, ScenarioInput> {
            scenarios
                .iter()
                .filter(|(_, s)| {
                    s.production_tasks.iter().any(|t| {
                        t.routing_ref.id == route.id && t.routing_ref.revision == route.revision
                    })
                })
                .map(|(id, s)| (id.clone(), s.clone()))
                .collect()
        }
        pub(super) fn scenario_applies(
            p: &ProjectState,
            scope: &[ObjectRef],
            s: &ScenarioInput,
        ) -> bool {
            selected(p, scope, &object(ObjectKind::Scenario, s.id.as_str()))
                || s.production_tasks.iter().any(|t| {
                    p.routes.iter().any(|route| {
                        route.id == t.routing_ref.id
                            && route.revision == t.routing_ref.revision
                            && selected(p, scope, &object(ObjectKind::Route, route.id.as_str()))
                    })
                })
        }
        fn stores(
            p: &ProjectState,
            d: &DeviceInstance,
            step: &RouteStep,
            op: &OperationDefinition,
            output: bool,
        ) -> Vec<(ObjectId, ObjectId)> {
            let mut result = vec![];
            for row in if output { &op.outputs } else { &op.inputs } {
                let item = step
                    .item_bindings
                    .get(&row.role)
                    .or_else(|| d.role_bindings.item_types.get(&row.role));
                let Some(item) = item else {
                    continue;
                };
                for c in &p.connections {
                    if let Connection::Material(c) = c {
                        if c.item_type == *item
                            && matches!(if output{&c.source}else{&c.destination},MaterialEndpoint::Port(r) if r.instance_id==d.id&&r.port_id==row.port&&same_module(&r.module_ref,&d.module_ref))
                        {
                            result.push((item.clone(), c.destination_store.clone()));
                        }
                    }
                }
            }
            result
        }
        fn graph(
            p: &ProjectState,
            scenarios: &BTreeMap<ScenarioId, ScenarioInput>,
            defs: &[ModuleDefinition],
            goal: &FactoryGenerationGoal,
            scope: &[ObjectRef],
            out: &mut Report,
        ) {
            for route in &p.routes {
                let r = object(ObjectKind::Route, route.id.as_str());
                if !selected(p, scope, &r) {
                    continue;
                }
                let route_context = route_scenarios(route, scenarios);
                for next in &route.steps {
                    let Some(nd) = p.devices.iter().find(|d| d.id == next.device_id) else {
                        continue;
                    };
                    let Some(no) = operation(defs, nd, Some(&next.operation_ref)) else {
                        continue;
                    };
                    for product in p.products.iter().filter(|v| {
                        v.routing_ref.id == route.id && v.routing_ref.revision == route.revision
                    }) {
                        if let Some(bom) = p.boms.iter().find(|b| {
                            b.id == product.bom_ref.id && b.revision == product.bom_ref.revision
                        }) {
                            for row in bom
                                .rows
                                .iter()
                                .filter(|b| b.consume_at_operation == next.id)
                            {
                                let actual: u128 = no
                                    .inputs
                                    .iter()
                                    .filter(|input| {
                                        next.item_bindings
                                            .get(&input.role)
                                            .or_else(|| {
                                                nd.role_bindings.item_types.get(&input.role)
                                            })
                                            .is_some_and(|id| *id == row.item_type_ref.id)
                                            && input.quantity.unit == row.quantity.unit
                                    })
                                    .map(|i| scaled(&i.quantity))
                                    .sum();
                                if actual != scaled(&row.quantity) {
                                    out.fail(K,&object(ObjectKind::Bom,bom.id.as_str()),"bom_operation_ratio","/rows/quantity","Actual selected operation material ratio disagrees with the exact product BOM row.");
                                }
                            }
                        }
                    }
                    let inputs = stores(p, nd, next, no, false);
                    for predecessor in &next.predecessors {
                        let Some(prev) = route.steps.iter().find(|s| s.id == *predecessor) else {
                            continue;
                        };
                        let Some(pd) = p.devices.iter().find(|d| d.id == prev.device_id) else {
                            continue;
                        };
                        let Some(po) = operation(defs, pd, Some(&prev.operation_ref)) else {
                            continue;
                        };
                        let outputs = stores(p, pd, prev, po, true);
                        for (item, dest) in &inputs {
                            let sources: Vec<_> =
                                outputs.iter().filter(|(i, _)| i == item).collect();
                            if !sources.is_empty()
                                && !sources
                                    .iter()
                                    .any(|(_, src)| linked(src, dest, item, &route_context))
                            {
                                out.fail(K,&r,"process_disconnected","/steps/predecessors",format!("Predecessor {} outputs {} to a different store with no declared transfer to {}.",prev.id.as_str(),item.as_str(),dest.as_str()));
                            }
                        }
                    }
                }
            }
            let root = object(ObjectKind::Project, p.id.as_str());
            let product = p.products.iter().find(|r| {
                r.id == goal.acceptance_scope.product_ref.id
                    && r.revision == goal.acceptance_scope.product_ref.revision
            });
            let route = product.and_then(|product| {
                p.routes.iter().find(|r| {
                    r.id == product.routing_ref.id && r.revision == product.routing_ref.revision
                })
            });
            if let (Some(product), Some(route)) = (product, route) {
                let mut outputs = vec![];
                for step in &route.steps {
                    if let Some(d) = p.devices.iter().find(|d| d.id == step.device_id) {
                        if let Some(op) = operation(defs, d, Some(&step.operation_ref)) {
                            outputs.extend(stores(p, d, step, op, true));
                        }
                    }
                }
                let sources: Vec<_> = outputs
                    .iter()
                    .filter(|(i, _)| *i == product.item_type_ref.id)
                    .collect();
                if sources.is_empty() {
                    out.missing(
                        K,
                        &root,
                        "acceptance_producer_missing",
                        "/generation_goal/acceptance_scope",
                        "No declared route output produces the selected accepted product item.",
                    );
                } else if !sources.iter().any(|(item, src)| {
                    linked(
                        src,
                        &goal.acceptance_scope.destination,
                        item,
                        &route_scenarios(route, scenarios),
                    )
                }) {
                    out.fail(K,&root,"acceptance_unreachable","/generation_goal/acceptance_scope/destination","Declared product outputs have no material/supply path to the requested acceptance location.");
                }
            }
        }
        pub(super) fn check_business(
            p: &ProjectState,
            scenarios: &BTreeMap<ScenarioId, ScenarioInput>,
            defs: &[ModuleDefinition],
            goal: &FactoryGenerationGoal,
            scope: &[ObjectRef],
            out: &mut Report,
        ) {
            let root = object(ObjectKind::Project, p.id.as_str());
            match &goal.production_calendar_ref {
                None => out.missing(
                    K,
                    &root,
                    "production_calendar",
                    "/generation_goal/production_calendar_ref",
                    "Production/shift calendar is missing; continuous production is not assumed.",
                ),
                Some(id) => calendar(
                    p,
                    id,
                    &root,
                    "/generation_goal/production_calendar_ref",
                    out,
                ),
            }
            if goal.accepted_quantity_per_day.is_none() {
                out.missing(
                    K,
                    &root,
                    "demand_missing",
                    "/generation_goal/accepted_quantity_per_day",
                    "Demand is absent; no target or capacity is inferred.",
                );
            }
            if goal.observation_window.end_ms.0.value()
                <= goal.observation_window.start_ms.0.value()
            {
                out.fail(
                    K,
                    &root,
                    "observation_window",
                    "/generation_goal/observation_window",
                    "Observation window must have positive duration.",
                );
            }
            for i in &goal.constraints {
                if !i.blocks.is_empty() {
                    out.row(
                        K,
                        CheckOutcome::Undetermined,
                        i.object_ref.clone().into_iter().collect(),
                        i.message.clone(),
                    );
                    out.issues.push(i.clone());
                }
            }
            for i in check_business_inputs_with_definitions(p, scenarios, defs) {
                if i.object_ref.as_ref().is_some_and(|r| {
                    selected(p, scope, r)
                        || matches!(r.kind, ObjectKind::Scenario)
                            && scenarios.values().any(|s| {
                                s.id.as_str() == r.id.as_str() && scenario_applies(p, scope, s)
                            })
                }) && !i.blocks.is_empty()
                {
                    let outcome = if unknown_issue(&i) {
                        CheckOutcome::Undetermined
                    } else {
                        CheckOutcome::Fail
                    };
                    out.row(
                        K,
                        outcome,
                        i.object_ref.clone().into_iter().collect(),
                        i.message.clone(),
                    );
                    out.issues.push(i);
                }
            }
            connection_checks(p, scenarios, defs, scope, K, out);
            let mut inspected = BTreeSet::new();
            for route in &p.routes {
                for step in &route.steps {
                    let Some(d) = p.devices.iter().find(|d| d.id == step.device_id) else {
                        continue;
                    };
                    let r = object(ObjectKind::Device, d.id.as_str());
                    if !selected(p, scope, &r) {
                        continue;
                    }
                    inspected.insert(d.id.clone());
                    if let Some(op) = operation(defs, d, Some(&step.operation_ref)) {
                        operation_checks(p, defs, d, Some(step), op, out);
                    } else if let Some(ModulePayload::Tasks(task)) =
                        exact(defs, &step.operation_ref).map(|d| &d.payload)
                    {
                        if task.steps.is_empty() {
                            duration_check(
                                task.duration.as_ref(),
                                &r,
                                "/preparation/duration",
                                out,
                            );
                            resources(
                                p,
                                d,
                                Some(step),
                                &task.resource_roles.iter().collect::<Vec<_>>(),
                                out,
                            );
                        } else {
                            for phase in &task.steps {
                                duration_check(
                                    phase.duration.as_ref(),
                                    &r,
                                    &format!("/preparation/steps/{}/duration", phase.id.as_str()),
                                    out,
                                );
                                resources(
                                    p,
                                    d,
                                    Some(step),
                                    &task
                                        .resource_roles
                                        .iter()
                                        .chain(&phase.resources)
                                        .collect::<Vec<_>>(),
                                    out,
                                );
                            }
                        }
                    } else {
                        out.missing(
                            K,
                            &r,
                            "behavior_missing",
                            "/operation_ref",
                            "Required exact execution/preparation behavior is unavailable.",
                        );
                    }
                }
            }
            for d in &p.devices {
                let r = object(ObjectKind::Device, d.id.as_str());
                if selected(p, scope, &r) && !inspected.contains(&d.id) {
                    if let Some(op) = operation(defs, d, None) {
                        operation_checks(p, defs, d, None, op, out);
                    } else {
                        out.missing(
                            K,
                            &r,
                            "behavior_missing",
                            "/operation_binding",
                            "No selected loaded execution behavior.",
                        );
                    }
                }
            }
            let mut relevant = false;
            for s in scenarios.values() {
                let sr = object(ObjectKind::Scenario, s.id.as_str());
                let applicable = scenario_applies(p, scope, s);
                if !applicable {
                    continue;
                }
                relevant = true;
                for task in &s.production_tasks {
                    if task.planned_quantity.value.coefficient.value() == 0
                        || task.planned_quantity.value.decimal_places.value() != 0
                    {
                        out.fail(K,&object(ObjectKind::Task,task.id.as_str()),"production_task_quantity","/planned_quantity","Discrete production task requires a positive integral declared quantity.");
                    }
                }

                if s.production_tasks.is_empty() {
                    out.missing(
                        K,
                        &sr,
                        "production_tasks_missing",
                        "/production_tasks",
                        "Empty production graph cannot prove material/process connectivity.",
                    );
                }
                for supply in &s.supplies {
                    let r = object(ObjectKind::Supply, supply.id.as_str());
                    if !p.locations.iter().any(|l| l.id == supply.source)
                        || !p.locations.iter().any(|l| l.id == supply.destination)
                    {
                        out.fail(
                            K,
                            &r,
                            "supply_location",
                            "/source",
                            "Supply endpoints must be actual locations.",
                        );
                    }
                    if !p.item_types.iter().any(|i| {
                        i.id == supply.item_type_ref.id
                            && i.revision == supply.item_type_ref.revision
                            && i.unit == supply.target_quantity.unit
                    }) {
                        out.fail(
                            K,
                            &r,
                            "supply_item_unit",
                            "/item_type_ref",
                            "Supply item revision and quantity unit must match.",
                        );
                    }
                    match exact(defs, &supply.transport_ref).map(|d| &d.payload) {
                        Some(ModulePayload::Transport(t)) => {
                            for (name, time) in [
                                ("loading", &t.times.loading),
                                ("outbound", &t.times.outbound),
                                ("unloading", &t.times.unloading),
                                ("returning", &t.times.returning),
                            ] {
                                duration_check(
                                    time.as_ref(),
                                    &r,
                                    &format!("/transport/times/{name}"),
                                    out,
                                );
                            }
                            if t.batch_capacity.is_none() {
                                out.missing(
                                    K,
                                    &r,
                                    "transport_capacity_missing",
                                    "/transport/batch_capacity",
                                    "Explicit transport batch capacity is missing.",
                                );
                            }
                            for req in t
                                .resources
                                .iter()
                                .chain(&t.loading_resources)
                                .chain(&t.unloading_resources)
                            {
                                match supply
                                    .resources
                                    .get(&req.role)
                                    .and_then(|id| p.resources.iter().find(|r| r.id == *id))
                                {
                                    None => out.missing(
                                        K,
                                        &r,
                                        "transport_resource_missing",
                                        "/resources",
                                        "Required transport resource is unbound or absent.",
                                    ),
                                    Some(resource) => {
                                        calendar(
                                            p,
                                            &resource.calendar_id,
                                            &r,
                                            "/resources/calendar_id",
                                            out,
                                        );
                                        if resource.capacity.unit != req.quantity.unit
                                            || scaled(&resource.capacity) < scaled(&req.quantity)
                                            || !req
                                                .qualifications
                                                .iter()
                                                .all(|q| resource.qualifications.contains(q))
                                        {
                                            out.fail(K,&r,"transport_resource","/resources","Transport resource units, quantity or qualifications are incompatible.");
                                        }
                                    }
                                }
                            }
                        }
                        _ => out.fail(
                            K,
                            &r,
                            "transport_definition",
                            "/transport_ref",
                            "Exact transport module is missing or wrong category.",
                        ),
                    }
                }
                out.pass_unless_problem(K,&sr,"Actual Scenario declarations checked; no jobs expanded and no inventory/time/output calculated.");
            }
            if !relevant {
                out.missing(
                    K,
                    &root,
                    "scenario_missing",
                    "/scenario_refs",
                    "No applicable actual production Scenario is available.",
                );
            }
            for source in &p.parameter_sources {
                if selected(p, scope, &source.target)
                    && matches!(source.basis, SourceBasis::Unknown)
                {
                    out.missing(
                        K,
                        &source.target,
                        "parameter_unknown",
                        &source.field_path,
                        "Declared execution parameter has unknown provenance/value.",
                    );
                }
            }
            graph(p, scenarios, defs, goal, scope, out);
            for d in &p.devices {
                let r = object(ObjectKind::Device, d.id.as_str());
                if selected(p, scope, &r) {
                    out.pass_unless_problem(K,&r,"Selected exact behavior, material endpoints, required resources, calendar and durations checked as declarations.");
                }
            }
            out.pass_unless_problem(K,&root,"Declared inputs/connectivity checked. Actual acceptance and target attainment require a separate Run.");
        }
    }
    mod parameters {
        //! Read-only effective-duration projection of the existing millisecond binding
        //! contract in simulation/parameters.rs. Only a local operation clone is edited.
        use super::*;

        pub(super) fn effective(
            p: &ProjectState,
            defs: &[ModuleDefinition],
            d: &DeviceInstance,
            reference: &ModuleRef,
            original: &OperationDefinition,
            out: &mut Report,
        ) -> OperationDefinition {
            use DraftCheckKind::MaterialProcessConnectivity as K;
            let mut operation = original.clone();
            let r = object(ObjectKind::Device, d.id.as_str());
            let Some(def) = exact(defs, &d.module_ref) else {
                return operation;
            };
            for (key, value) in &d.configuration {
                let path = format!(
                    "/configuration/{}/value",
                    key.replace('~', "~0").replace('/', "~1")
                );
                let declarations: Vec<_> = def
                    .parameters
                    .iter()
                    .filter(|p| p.id.as_str() == key)
                    .collect();
                if declarations.len() != 1 {
                    out.fail(K,&r,"parameter_declaration",&path,"Configuration key needs one exact parameter declaration; it is not a cycle-time alias.");
                    continue;
                }
                let parameter = declarations[0];
                let sources: Vec<_> = p
                    .parameter_sources
                    .iter()
                    .filter(|s| same_object(&s.target, &r) && s.field_path == path)
                    .collect();
                if sources.len() > 1
                    || sources.first().is_some_and(|s| {
                        s.value
                            .as_ref()
                            .is_none_or(|v| format!("{v:?}") != format!("{value:?}"))
                    })
                {
                    out.fail(
                        K,
                        &r,
                        "parameter_source",
                        &path,
                        "Stored parameter source does not name this actual configuration value.",
                    );
                    continue;
                }
                if matches!(
                    (
                        key.as_str(),
                        parameter.binding.as_str(),
                        &parameter.value_type,
                        value
                    ),
                    (
                        "packaging_job_id",
                        "/payload/data/configuration/packaging_job_id/value",
                        ValueKind::String,
                        TypedValue::String(_)
                    )
                ) || parameter.integer
                    && matches!(
                        (
                            key.as_str(),
                            parameter.binding.as_str(),
                            &parameter.value_type,
                            value
                        ),
                        (
                            "packaging_job_revision",
                            "/payload/data/configuration/packaging_job_revision/value",
                            ValueKind::Integer,
                            TypedValue::Integer(_)
                        )
                    )
                {
                    continue;
                }
                if !matches!(
                    (&parameter.value_type, value),
                    (ValueKind::Integer, TypedValue::Integer(_))
                        | (ValueKind::Number, TypedValue::Number(_))
                        | (ValueKind::Duration, TypedValue::Duration(_))
                ) {
                    out.fail(
                        K,
                        &r,
                        "parameter_type",
                        &path,
                        "Execution duration requires its declared numeric/duration type.",
                    );
                    continue;
                }
                let n = match value {
                    TypedValue::Integer(n) => n.value() as f64,
                    TypedValue::Number(n) => n.value(),
                    TypedValue::Duration(d) => d.value.0.value() as f64,
                    _ => unreachable!(),
                };
                if n < 0.
                    || n > MAX_SAFE_UINT as f64
                    || n.fract() != 0.
                    || parameter.minimum.as_ref().is_some_and(|x| n < x.value())
                    || parameter.maximum.as_ref().is_some_and(|x| n > x.value())
                    || parameter
                        .unit
                        .as_ref()
                        .is_some_and(|u| u.as_str() != "millisecond")
                {
                    out.fail(K,&r,"parameter_range_unit",&path,"Duration must be in declared bounds and exact nonnegative canonical integer milliseconds.");
                    continue;
                }
                let Some(binding) = parameter
                    .binding
                    .strip_prefix("/payload/data/behavior/inline/")
                else {
                    out.fail(
                K,
                &r,
                "parameter_binding_scope",
                &path,
                "Unsupported execution binding; the exact declared RFC6901 path is required.",
            );
                    continue;
                };
                if !same_module(reference, &d.module_ref) {
                    out.fail(
                        K,
                        &r,
                        "parameter_inactive",
                        &path,
                        "Inline parameter is not the selected operation for this route step.",
                    );
                    continue;
                }
                let (target, whole) = if binding == "duration" {
                    (operation.duration.as_mut(), true)
                } else if binding == "duration/value" {
                    (operation.duration.as_mut(), false)
                } else {
                    let parts: Vec<_> = binding.split('/').collect();
                    if parts.first() == Some(&"phases")
                        && matches!(parts.len(), 3 | 4)
                        && parts[2] == "duration"
                        && (parts.len() == 3 || parts[3] == "value")
                    {
                        (
                            parts[1]
                                .parse::<usize>()
                                .ok()
                                .and_then(|i| operation.phases.get_mut(i))
                                .and_then(|p| p.duration.as_mut()),
                            parts.len() == 3,
                        )
                    } else {
                        (None, false)
                    }
                };
                let Some(target) = target else {
                    out.fail(
                        K,
                        &r,
                        "parameter_binding_missing",
                        &path,
                        "Declared duration/phase binding does not exist in the effective rule.",
                    );
                    continue;
                };
                let basis = sources
                    .first()
                    .map(|s| s.basis.clone())
                    .unwrap_or(SourceBasis::Unknown);
                if whole {
                    let TypedValue::Duration(duration) = value else {
                        out.fail(
                            K,
                            &r,
                            "parameter_binding_type",
                            &path,
                            "Whole duration binding requires a typed DurationValue.",
                        );
                        continue;
                    };
                    *target = duration.clone();
                    target.basis = basis;
                } else {
                    if matches!(value, TypedValue::Duration(_)) {
                        out.fail(
                            K,
                            &r,
                            "parameter_binding_type",
                            &path,
                            "Scalar duration value cannot be replaced with a whole DurationValue.",
                        );
                        continue;
                    }
                    target.value = DurationMs(SafeUint::try_from(n as u64).unwrap());
                    target.basis = basis;
                    target.source_ref = sources.first().and_then(|s| s.source_ref.clone());
                }
            }
            operation
        }
    }
}
