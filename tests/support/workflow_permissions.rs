use serde_yaml::{Mapping, Value};

pub(crate) fn assert_release_write_permissions_are_trusted(workflow: &str) -> Result<(), String> {
    let document = serde_yaml::from_str::<Value>(workflow)
        .map_err(|error| format!("workflow must be valid YAML: {error}"))?;
    let root = mapping(&document, "workflow")?;
    let top_permissions = mapping_field(root, "permissions", "workflow")?;
    require_exact_permission(top_permissions, "contents", "read", "top-level")?;
    let jobs = mapping_field(root, "jobs", "workflow")?;
    let mut checkout_count = 0;
    for (name, job) in jobs {
        let job_name = name
            .as_str()
            .ok_or_else(|| "workflow job names must be strings".to_owned())?;
        let job = mapping(job, job_name)?;
        let permissions = field(job, "permissions");
        if matches!(job_name, "publish-release" | "finalize-release") {
            let permissions =
                permissions.ok_or_else(|| format!("{job_name} permissions missing"))?;
            let permissions = mapping(permissions, &format!("{job_name} permissions"))?;
            require_exact_permission(permissions, "contents", "write", job_name)?;
            require_trusted_release_condition(job)?;
        } else if job_name == "publish-runtime-tool" {
            let permissions = permissions.ok_or("runtime-tool permissions missing".to_owned())?;
            let permissions = mapping(permissions, "publish-runtime-tool permissions")?;
            require_exact_permission(permissions, "id-token", "write", "publish-runtime-tool")?;
            require_trusted_release_condition(job)?;
        } else if let Some(permissions) = permissions {
            reject_write_permissions(permissions, job_name)?;
        }
        let Some(steps) = field(job, "steps") else {
            continue;
        };
        let steps = steps
            .as_sequence()
            .ok_or_else(|| format!("{job_name} steps must be a sequence"))?;
        for step in steps {
            let step = mapping(step, "workflow step")?;
            if !field(step, "uses")
                .and_then(Value::as_str)
                .is_some_and(|uses| uses.starts_with("actions/checkout@"))
            {
                continue;
            }
            checkout_count += 1;
            let inputs = mapping_field(step, "with", "checkout step")?;
            let credential_value =
                canonical_field(inputs, "persist-credentials").ok_or_else(|| {
                    format!(
                        "checkout {checkout_count} must set exactly one persist-credentials: false"
                    )
                })?;
            if credential_value != &Value::Bool(false) {
                return Err(format!(
                    "checkout {checkout_count} must set persist-credentials: false"
                ));
            }
        }
    }
    if checkout_count == 0 {
        return Err("workflow must contain at least one checkout".to_owned());
    }
    Ok(())
}

fn mapping<'a>(value: &'a Value, context: &str) -> Result<&'a Mapping, String> {
    value
        .as_mapping()
        .ok_or_else(|| format!("{context} must be a mapping"))
}

fn field<'a>(mapping: &'a Mapping, name: &str) -> Option<&'a Value> {
    mapping.get(Value::String(name.to_owned()))
}

fn canonical_field<'a>(mapping: &'a Mapping, name: &str) -> Option<&'a Value> {
    let mut matches = mapping.iter().filter(|(key, _)| {
        key.as_str()
            .is_some_and(|key| key.eq_ignore_ascii_case(name))
    });
    let (key, value) = matches.next()?;
    (matches.next().is_none() && key.as_str() == Some(name)).then_some(value)
}

fn mapping_field<'a>(
    parent: &'a Mapping,
    name: &str,
    context: &str,
) -> Result<&'a Mapping, String> {
    let value = field(parent, name).ok_or_else(|| format!("{context} must define {name}"))?;
    mapping(value, name)
}

fn require_exact_permission(
    permissions: &Mapping,
    permission: &str,
    level: &str,
    context: &str,
) -> Result<(), String> {
    if permissions.len() != 1
        || field(permissions, permission).and_then(Value::as_str) != Some(level)
    {
        return Err(format!(
            "{context} permissions must be exactly {permission}: {level}"
        ));
    }
    Ok(())
}

fn reject_write_permissions(permissions: &Value, job_name: &str) -> Result<(), String> {
    let permissions = mapping(permissions, job_name)?;
    for (permission, level) in permissions {
        if level.as_str().is_some_and(|level| {
            level.eq_ignore_ascii_case("write") || level.eq_ignore_ascii_case("write-all")
        }) {
            return Err(format!(
                "only trusted publish jobs may receive write permissions; {job_name} grants {permission:?}"
            ));
        }
    }
    Ok(())
}

fn require_trusted_release_condition(job: &Mapping) -> Result<(), String> {
    let condition = field(job, "if")
        .and_then(Value::as_str)
        .ok_or_else(|| "publish-release must retain its trusted release condition".to_owned())?;
    (condition == "startsWith(github.ref, 'refs/tags/')")
        .then_some(())
        .ok_or_else(|| "publish-release must retain its trusted release condition".to_owned())
}
