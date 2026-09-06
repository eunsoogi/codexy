use std::{fs, path::{Path, PathBuf}, process::Command};

use serde_json::Value;

use super::direct_state;
use crate::support::TestResult;

const REPAIR_PATH: &str =
    "packages/codexy-runtime/src/validation/child_goal_reporting/receipt/parse.rs";
const EXTERNAL_FINDING_PATH: &str = "packages/codexy-runtime/src/validation/review_control/state.rs";
const DISPOSITION_REPAIR_PATH: &str =
    "packages/codexy-runtime/src/validation/review_control/external_finding/capture.rs";

pub(crate) struct SyntheticRepository {
    pub(crate) path: PathBuf,
    base: String,
    updated_base: String,
    full: String,
    delta: String,
    integration_evidence: String,
    integration_current: String,
    repair_evidence: String,
    repair_current: String,
    external_evidence: String,
    external_current: String,
    disposition_evidence: String,
    disposition_current: String,
}

impl SyntheticRepository {
    pub(crate) fn create(root: &Path) -> TestResult<Self> {
        let path = root.join("repository");
        fs::create_dir_all(&path)?;
        git(&path, &["init", "--quiet"])?;
        git(&path, &["config", "user.name", "codexy-test"])?;
        git(&path, &["config", "user.email", "codexy-test@example.test"])?;
        git(&path, &["config", "commit.gpgSign", "false"])?;

        write(&path, "base.txt", "base\n")?;
        let base = commit(&path, "base")?;
        write(&path, "full.txt", "full\n")?;
        let full = commit(&path, "full review")?;
        write(&path, "delta.txt", "delta\n")?;
        let delta = commit(&path, "delta review")?;

        git(&path, &["switch", "--create", "base-after", &base])?;
        write(&path, "base-integration.txt", "integrated\n")?;
        let updated_base = commit(&path, "mandatory base integration")?;

        git(&path, &["switch", "--create", "integration", &delta])?;
        git(&path, &["merge", "--no-edit", "--no-ff", "base-after"])?;
        let integration_evidence = head(&path)?;
        write(&path, "integration-current.txt", "current\n")?;
        let integration_current = commit(&path, "current integration head")?;

        git(&path, &["switch", "--create", "repair", &delta])?;
        write(&path, REPAIR_PATH, "repaired\n")?;
        let repair_evidence = commit(&path, "in-scope root repair")?;
        write(&path, "repair-current.txt", "current\n")?;
        let repair_current = commit(&path, "current repair head")?;

        git(&path, &["switch", "--create", "external", &delta])?;
        write(&path, EXTERNAL_FINDING_PATH, "repaired external finding\n")?;
        let external_evidence = commit(&path, "authenticated external finding repair")?;
        write(&path, "external-current.txt", "current\n")?;
        let external_current = commit(&path, "current external finding head")?;

        git(&path, &["switch", "--create", "disposition", &delta])?;
        write(&path, DISPOSITION_REPAIR_PATH, "repaired disposition finding\n")?;
        let disposition_evidence = commit(&path, "authenticated finding disposition repair")?;
        write(&path, "disposition-current.txt", "current\n")?;
        let disposition_current = commit(&path, "current disposition head")?;

        Ok(Self {
            path,
            base,
            updated_base,
            full,
            delta,
            integration_evidence,
            integration_current,
            repair_evidence,
            repair_current,
            external_evidence,
            external_current,
            disposition_evidence,
            disposition_current,
        })
    }

    pub(crate) fn prepare(
        &self,
        control: &Value,
        previous_base: &str,
        current_base: &str,
    ) -> TestResult<(Value, String, String)> {
        let mut control = control.clone();
        let root_repair = control["post_cap_re_review"]["reason"].as_str()
            == Some("in_scope_contract_root_repair");
        let external_finding = control["post_cap_re_review"]["reason"].as_str()
            == Some("authenticated_external_finding_repair");
        let disposition = control["post_cap_re_review"]["reason"].as_str()
            == Some("authenticated_finding_disposition");
        rewrite(&mut control, self, root_repair, external_finding, disposition);
        Ok((
            control,
            self.map_oid(previous_base)?,
            self.map_oid(current_base)?,
        ))
    }

    fn map_oid(&self, value: &str) -> TestResult<String> {
        self.resolve(value, false, false, false)
    }

    pub(crate) fn resolve(
        &self,
        value: &str,
        root_repair: bool,
        external_finding: bool,
        disposition: bool,
    ) -> TestResult<String> {
        self.map(value, root_repair, external_finding, disposition)
            .map(str::to_owned)
            .ok_or_else(|| format!("unknown synthetic review reference: {value}").into())
    }

    fn map<'a>(
        &'a self,
        value: &str,
        root_repair: bool,
        external_finding: bool,
        disposition: bool,
    ) -> Option<&'a str> {
        match value {
            direct_state::SYNTHETIC_BASE => Some(&self.base),
            direct_state::SYNTHETIC_UPDATED_BASE => Some(&self.updated_base),
            direct_state::SYNTHETIC_FULL_HEAD => Some(&self.full),
            direct_state::SYNTHETIC_DELTA_HEAD => Some(&self.delta),
            direct_state::SYNTHETIC_CURRENT_HEAD => {
                if root_repair {
                    Some(&self.repair_current)
                } else if external_finding {
                    Some(&self.external_current)
                } else if disposition {
                    Some(&self.disposition_current)
                } else {
                    Some(&self.integration_current)
                }
            }
            direct_state::SYNTHETIC_INTEGRATION_EVIDENCE => Some(&self.integration_evidence),
            direct_state::SYNTHETIC_REPAIR_EVIDENCE => Some(&self.repair_evidence),
            direct_state::SYNTHETIC_EXTERNAL_EVIDENCE => Some(&self.external_evidence),
            direct_state::SYNTHETIC_DISPOSITION_EVIDENCE => Some(&self.disposition_evidence),
            _ => None,
        }
    }
}

fn rewrite(
    value: &mut Value,
    repository: &SyntheticRepository,
    root_repair: bool,
    external_finding: bool,
    disposition: bool,
) {
    match value {
        Value::String(text) => {
            if let Some(mapped) = repository.map(text, root_repair, external_finding, disposition) {
                *text = mapped.to_owned();
            }
        }
        Value::Array(values) => values
            .iter_mut()
            .for_each(|value| rewrite(value, repository, root_repair, external_finding, disposition)),
        Value::Object(values) => values
            .values_mut()
            .for_each(|value| rewrite(value, repository, root_repair, external_finding, disposition)),
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn write(root: &Path, relative: &str, contents: &str) -> TestResult<()> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

fn commit(root: &Path, message: &str) -> TestResult<String> {
    git(root, &["add", "--", "."])?;
    git(root, &["commit", "--quiet", "-m", message])?;
    head(root)
}

fn head(root: &Path) -> TestResult<String> { git(root, &["rev-parse", "HEAD"]) }

fn git(root: &Path, args: &[&str]) -> TestResult<String> {
    let output = Command::new("git").current_dir(root).args(args).output()?;
    if !output.status.success() {
        return Err(format!(
            "synthetic review repository git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}
