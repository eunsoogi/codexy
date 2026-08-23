use std::collections::BTreeMap;

use super::schema::{
    Classification, Contract, CurrentState, Envelope, RetainedField, Slot, classification,
    slot_is_valid,
};

pub(super) fn validate_envelope(
    contract: &Contract,
    envelope: &Envelope,
    current: &CurrentState,
    expected_identities: &[String; 2],
) -> Vec<String> {
    let mut errors = Vec::new();
    let classification = current
        .slots
        .get("task_classification")
        .and_then(|slot| classification(slot, contract));
    let Some(classification) = classification else {
        errors.push("task classification is not a closed workflow/surface/risk value".to_owned());
        return errors;
    };
    let known_workflow = contract
        .routing
        .task_classes
        .contains(&classification.workflow)
        || contract
            .routing
            .fail_closed_classes
            .contains(&classification.workflow);
    let known_surfaces = classification
        .surfaces
        .iter()
        .all(|surface| contract.routing.surface_names.contains(surface));
    let known_risks = classification
        .risks
        .iter()
        .all(|risk| contract.routing.risk_names.contains(risk));
    let fail_closed = !known_workflow
        || !known_surfaces
        || !known_risks
        || classification.surfaces.is_empty()
        || !classification.risks.is_empty()
        || contract
            .routing
            .fail_closed_classes
            .contains(&classification.workflow);
    if envelope.schema != "codexy.context-envelope.v1" {
        errors.push("context envelope or current state has an unsupported schema".to_owned());
    }
    if classification.workflow != envelope.task_class {
        errors.push("context envelope classification slots disagree with its routing".to_owned());
    }
    if fail_closed
        && (envelope.profile != "strict"
            || envelope.route_authority.as_deref()
                != Some(contract.routing.fallback_authority.as_str())
            || envelope.action_allowed)
    {
        errors.push(
            "risk or unresolved context must fail closed through the routing authority".to_owned(),
        );
    }
    if !fail_closed && envelope.route_authority.is_some() {
        errors.push("ordinary context must not invent a risk-route authority".to_owned());
    }
    let expected = expected_references(contract, &classification, fail_closed);
    if !selected_refs(&envelope.slots, &expected) {
        errors.push("selected references disagree with the closed surface route".to_owned());
    }
    if !contract.profile_matrix.contains_key(&envelope.profile) {
        errors.push("context envelope has an unknown workflow profile".to_owned());
    }
    if slot_string(&envelope.slots, "workflow_profile") != Some(envelope.profile.as_str()) {
        errors.push("context envelope profile slot disagrees with its routing".to_owned());
    }
    if envelope.stable_identity != expected_identities[0]
        || envelope.volatile_identity != expected_identities[1]
    {
        errors.push("context envelope identities do not match deterministic state".to_owned());
    }
    for field in &contract.retained_fields {
        compare(
            contract,
            envelope,
            current,
            &classification,
            field,
            &mut errors,
        );
    }
    if envelope.slots.keys().any(|name| {
        !contract
            .retained_fields
            .iter()
            .any(|field| field.name == *name)
    }) {
        errors.push("context state contains an unknown retained field".to_owned());
    }
    errors
}

fn expected_references(
    contract: &Contract,
    classification: &Classification,
    fail_closed: bool,
) -> Vec<String> {
    let mut route = Vec::new();
    let add = |route: &mut Vec<String>, values: &[String]| {
        for value in values {
            if !route.contains(value) {
                route.push(value.clone());
            }
        }
    };
    if fail_closed {
        add(&mut route, &contract.routing.fallback_reference_route);
    } else if let Some(values) = contract
        .routing
        .task_reference_routes
        .get(&classification.workflow)
    {
        add(&mut route, values);
    }
    if !fail_closed {
        for surface in &classification.surfaces {
            if let Some(values) = contract.routing.surface_reference_routes.get(surface) {
                add(&mut route, values);
            }
        }
    }
    for risk in &classification.risks {
        if let Some(values) = contract.routing.risk_reference_routes.get(risk) {
            add(&mut route, values);
        }
    }
    route
}

fn compare(
    contract: &Contract,
    envelope: &Envelope,
    current: &CurrentState,
    classification: &Classification,
    field: &RetainedField,
    errors: &mut Vec<String>,
) {
    let Some((retained, authoritative)) = envelope
        .slots
        .get(&field.name)
        .zip(current.slots.get(&field.name))
    else {
        errors.push(format!("missing retained field {}", field.name));
        return;
    };
    if retained != authoritative {
        errors.push(format!("stale retained field {}", field.name));
    }
    if !slot_is_valid(contract, &field.name, authoritative) {
        errors.push(format!("invalid value shape for {}", field.name));
    }
    let Slot::Omitted(omission) = retained else {
        return;
    };
    let policy = contract
        .profile_matrix
        .get(&envelope.profile)
        .and_then(|tiers| tiers.get(&field.tier))
        .map_or("invalid", String::as_str);
    let typed = contract.omission_reasons.contains(&omission.omitted.code)
        && !omission.omitted.reason.trim().is_empty();
    let surface_omission = field.safety_invariant
        && !classification.surfaces.iter().any(|surface| {
            contract
                .routing
                .surface_non_applicable_fields
                .get(surface)
                .is_none_or(|fields| !fields.contains(&field.name))
        });
    let permitted = match policy {
        "when_applicable" | "typed_omission_when_not_applicable" => typed,
        "required_before_authoritative_action" => typed && !envelope.action_allowed,
        "required" => typed && surface_omission,
        _ => false,
    };
    if !permitted {
        errors.push(format!("unauthorized omission for {}", field.name));
    }
    if omission.omitted.code == "external_surface_absent" && envelope.action_allowed {
        errors.push("unavailable external state must fail closed".to_owned());
    }
}

fn selected_refs(slots: &BTreeMap<String, Slot>, expected: &[String]) -> bool {
    let Some(Slot::Present(present)) = slots.get("selected_references") else {
        return false;
    };
    present.value.as_array().is_some_and(|items| {
        items.len() == expected.len()
            && items
                .iter()
                .zip(expected)
                .all(|(item, name)| item.as_str() == Some(name))
    })
}

fn slot_string<'a>(slots: &'a BTreeMap<String, Slot>, name: &str) -> Option<&'a str> {
    match slots.get(name) {
        Some(Slot::Present(present)) => present.value.as_str(),
        _ => None,
    }
}
