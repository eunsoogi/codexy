use std::path::Path;

use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct Manifest {
    pub(super) baseline: String,
    pub(super) mappings: Vec<Mapping>,
}

#[derive(Deserialize)]
pub(super) struct Mapping {
    pub(super) source: String,
    pub(super) destination: String,
    pub(super) entrypoint: String,
    pub(super) identity_file: String,
}

#[derive(Deserialize)]
pub(super) struct IdentityFile {
    pub(super) source: String,
    pub(super) identities: Vec<String>,
}

pub(super) fn manifest(path: &Path, errors: &mut Vec<String>) -> Option<Manifest> {
    load(path, errors)
}

pub(super) fn identity(path: &Path, errors: &mut Vec<String>) -> Option<IdentityFile> {
    load(path, errors)
}

fn load<T: serde::de::DeserializeOwned>(path: &Path, errors: &mut Vec<String>) -> Option<T> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            errors.push(format!("{}: {error}", path.display()));
            return None;
        }
    };
    match serde_json::from_str(&text) {
        Ok(value) => Some(value),
        Err(error) => {
            errors.push(format!("{}: {error}", path.display()));
            None
        }
    }
}
