pub(crate) fn json_array_strings(value: Option<&serde_json::Value>) -> Option<Vec<String>> {
    value
        .and_then(serde_json::Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .map(serde_json::Value::as_str)
                .collect::<Option<Vec<_>>>()
                .map(|strings| strings.into_iter().map(ToOwned::to_owned).collect())
        })
}

pub(crate) fn toml_array_strings(value: Option<&toml::Value>) -> Option<Vec<String>> {
    value.and_then(toml::Value::as_array).and_then(|items| {
        items
            .iter()
            .map(toml::Value::as_str)
            .collect::<Option<Vec<_>>>()
            .map(|strings| strings.into_iter().map(ToOwned::to_owned).collect())
    })
}
