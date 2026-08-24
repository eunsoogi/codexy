use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Result, bail};
use serde_json::Value;
use sha2::{Digest as _, Sha256};

const PACKAGE_REL: &str = "skills/orchestration/references/review-economics";
const CORPUS_REL: &str = "skills/orchestration/references/review-economics-corpus.json";

#[derive(Debug)]
pub(super) struct Package {
    pub(super) manifest_sha256: String,
    pub(super) baseline_sha256: String,
    pub(super) corpus_sha256: String,
    pub(super) seeds: BTreeMap<String, SeedSpec>,
    pub(super) lanes: Vec<LaneSpec>,
}

#[derive(Debug)]
pub(super) struct SeedSpec {
    pub(super) path: PathBuf,
    pub(super) sha256: String,
    pub(super) severity: String,
}

#[derive(Debug)]
pub(super) struct LaneSpec {
    pub(super) id: String,
    pub(super) kind: String,
    pub(super) profile: String,
    pub(super) path: PathBuf,
    pub(super) sha256: String,
    pub(super) seed_ids: Vec<String>,
}

pub(super) fn load(plugin_root: &Path) -> Result<Package> {
    let root = plugin_root.join(PACKAGE_REL);
    let manifest_bytes = fs::read(root.join("manifest.json"))?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    if manifest["schema"] != "codexy.review-economics-package.v1"
        || manifest["version"] != "1.0.0"
        || manifest["corpus"] != "../review-economics-corpus.json"
        || manifest["baseline"] != "baseline-pre-1.5.json"
        || manifest["capture_protocol"] != "codexy.review-economics-capture.v1"
    {
        bail!("review economics package manifest is not the frozen v1 contract");
    }
    let baseline_path = checked_child(&root, string(&manifest, "baseline")?)?;
    let baseline_bytes = fs::read(&baseline_path)?;
    validate_baseline(&serde_json::from_slice(&baseline_bytes)?)?;
    let corpus_path = plugin_root.join(CORPUS_REL);
    let corpus_bytes = fs::read(&corpus_path)?;
    let corpus: Value = serde_json::from_slice(&corpus_bytes)?;
    validate_corpus(&corpus)?;
    let seed_refs = manifest["seeds"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("package seeds are missing"))?;
    if seed_refs.len() != 3 {
        bail!("review economics package must contain three seeds");
    }
    let mut seeds = BTreeMap::new();
    for reference in seed_refs {
        let id = string(reference, "id")?.to_owned();
        let version = string(reference, "version")?;
        let severity = string(reference, "severity")?.to_owned();
        let path = checked_child(&root, string(reference, "path")?)?;
        let bytes = fs::read(&path)?;
        let seed: Value = serde_json::from_slice(&bytes)?;
        let findings = seed["expected_findings"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("seed findings are missing"))?;
        if seed["schema"] != "codexy.review-economics-seed.v1"
            || seed["id"] != id
            || string(&seed, "version")? != version
            || seed["severity"] != severity
            || seed["payload"].is_null()
            || findings.len() != 1
            || findings.iter().any(|finding| {
                finding["id"] != id
                    || finding["severity"] != severity
                    || finding["required"] != true
            })
        {
            bail!("review economics seed payload or expected finding is invalid");
        }
        if seeds
            .insert(
                id,
                SeedSpec {
                    path,
                    sha256: digest(&bytes),
                    severity,
                },
            )
            .is_some()
        {
            bail!("review economics seed ids must be unique");
        }
    }
    let lane_refs = manifest["lanes"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("package lanes are missing"))?;
    if lane_refs.len() != 5 {
        bail!("review economics package must contain five lanes");
    }
    let mut lanes = Vec::new();
    for reference in lane_refs {
        let path = checked_child(&root, string(reference, "path")?)?;
        let bytes = fs::read(&path)?;
        let input: Value = serde_json::from_slice(&bytes)?;
        let seeds_in_input = ids(&input["seeds"])?;
        let seeds_in_manifest = ids(&reference["seeds"])?;
        let id = string(reference, "id")?.to_owned();
        let version = string(reference, "version")?;
        let corpus_lane = corpus["lanes"]
            .as_array()
            .and_then(|items| items.iter().find(|item| item["id"] == id))
            .ok_or_else(|| anyhow::anyhow!("lane input is absent from the acceptance corpus"))?;
        if input["schema"] != "codexy.review-economics-lane-input.v1"
            || input["id"] != id
            || string(&input, "version")? != version
            || input["kind"] != reference["kind"]
            || input["profile"] != reference["profile"]
            || seeds_in_input != seeds_in_manifest
            || input["request"].is_null()
            || corpus_lane["kind"] != input["kind"]
            || corpus_lane["profile"] != input["profile"]
            || ids(&corpus_lane["seeds"])? != seeds_in_input
            || seeds_in_input
                .iter()
                .any(|seed_id| seeds.get(seed_id).is_none())
        {
            bail!("review economics lane input is not bound to its manifest and corpus");
        }
        if lanes.iter().any(|item: &LaneSpec| item.id == id) {
            bail!("review economics lane ids must be unique");
        }
        lanes.push(LaneSpec {
            id,
            kind: string(&input, "kind")?.to_owned(),
            profile: string(&input, "profile")?.to_owned(),
            path,
            sha256: digest(&bytes),
            seed_ids: seeds_in_input,
        });
    }
    let lane_ids = lanes
        .iter()
        .map(|lane| lane.id.as_str())
        .collect::<BTreeSet<_>>();
    let corpus_ids = corpus["lanes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|lane| lane["id"].as_str())
        .collect::<BTreeSet<_>>();
    if lane_ids.len() != 5 || lane_ids != corpus_ids {
        bail!("review economics package and corpus lanes disagree");
    }
    Ok(Package {
        manifest_sha256: digest(&manifest_bytes),
        baseline_sha256: digest(&baseline_bytes),
        corpus_sha256: digest(&corpus_bytes),
        seeds,
        lanes,
    })
}

fn validate_baseline(value: &Value) -> Result<()> {
    let fields = value["unavailable_fields"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("baseline unavailable fields are missing"))?;
    let required = [
        "head_oid",
        "tree_oid",
        "policy_sha256",
        "corpus_sha256",
        "tokens",
        "cost",
        "runtime_telemetry",
        "reviewer_receipts",
    ];
    if value["schema"] != "codexy.review-economics-baseline.v1"
        || value["version"] != "pre-1.5"
        || value["status"] != "frozen"
        || value["provenance"].is_null()
        || value["lanes"]
            .as_array()
            .map_or(true, |items| items.len() != 5)
        || required
            .iter()
            .any(|name| !fields.iter().any(|field| field.as_str() == Some(name)))
        || string(&value["provenance"], "source").is_err()
    {
        bail!("review economics baseline must remain frozen with explicit unavailable fields");
    }
    Ok(())
}

fn validate_corpus(value: &Value) -> Result<()> {
    let lanes = value["lanes"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("review economics corpus lanes are missing"))?;
    if value["schema"] != "codexy.review-economics-corpus.v1"
        || lanes.len() != 5
        || lanes.iter().any(|lane| lane["id"].as_str().is_none())
    {
        bail!("review economics corpus is not the five-lane v1 contract");
    }
    Ok(())
}

fn ids(value: &Value) -> Result<Vec<String>> {
    Ok(value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("package seed ids are missing"))?
        .iter()
        .map(|item| string(item, "id").map(str::to_owned))
        .collect::<Result<_>>()?)
}

fn checked_child(root: &Path, relative: &str) -> Result<PathBuf> {
    if relative.is_empty()
        || Path::new(relative).is_absolute()
        || relative
            .split('/')
            .any(|part| matches!(part, "" | "." | ".."))
    {
        bail!("review economics package path is unsafe");
    }
    Ok(root.join(relative))
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .filter(|text| !text.is_empty())
        .ok_or_else(|| anyhow::anyhow!("missing string field: {key}"))
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
