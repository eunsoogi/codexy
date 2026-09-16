use std::collections::BTreeMap;
use std::path::Path;

const COMPONENT_ROOTS: &[(&str, &str)] = &[
    ("core", "plugins/codexy/skills"),
    ("github", "plugins/codexy-github/skills"),
    ("devtools", "plugins/codexy-devtools/skills"),
];

#[derive(Debug, Eq, PartialEq)]
pub(super) struct Skill {
    pub(super) component: String,
    pub(super) link: String,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct DocumentedSkill {
    pub(super) purpose: String,
    pub(super) component: String,
    pub(super) link: String,
}

pub(super) fn packaged(root: &Path) -> Result<BTreeMap<String, Skill>, String> {
    let mut skills = BTreeMap::new();
    for (component, relative_root) in COMPONENT_ROOTS {
        for entry in
            std::fs::read_dir(root.join(relative_root)).map_err(|error| error.to_string())?
        {
            let entry = entry.map_err(|error| error.to_string())?;
            let directory = entry.path();
            let skill_path = directory.join("SKILL.md");
            if !skill_path.is_file() {
                continue;
            }
            let name = frontmatter_name(&skill_path)?;
            let link = format!(
                "{relative_root}/{}/SKILL.md",
                entry.file_name().to_string_lossy()
            );
            if skills
                .insert(
                    name.clone(),
                    Skill { component: (*component).into(), link },
                )
                .is_some()
            {
                return Err(format!("duplicate packaged skill: {name}"));
            }
        }
    }
    if skills.is_empty() {
        return Err("no packaged skills discovered".into());
    }
    Ok(skills)
}

fn frontmatter_name(path: &Path) -> Result<String, String> {
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let frontmatter = text.split("---").nth(1).ok_or("skill frontmatter missing")?;
    let value: serde_yaml::Value =
        serde_yaml::from_str(frontmatter).map_err(|error| error.to_string())?;
    value["name"]
        .as_str()
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("skill name missing in {}", path.display()))
}

pub(super) fn documented(
    root: &Path,
    text: &str,
    expected: &BTreeMap<String, Skill>,
    source: &str,
) -> Result<BTreeMap<String, DocumentedSkill>, String> {
    let rows = inventory_rows(text)?;
    let mut documented = BTreeMap::new();
    for row in rows {
        if documented.insert(row_name(&row)?, row_value(&row)?).is_some() {
            return Err(format!("duplicate documented skill in {source}"));
        }
    }
    if documented.keys().collect::<Vec<_>>() != expected.keys().collect::<Vec<_>>() {
        return Err(format!("documented skill set differs in {source}"));
    }
    for (name, skill) in expected {
        let row = documented
            .get(name)
            .ok_or_else(|| format!("missing skill {name}"))?;
        if !Path::new(&row.link).is_relative() || !root.join(&row.link).is_file() {
            return Err(format!("skill {name} has a broken local link in {source}"));
        }
        if row.component != skill.component {
            return Err(format!("skill {name} has the wrong component in {source}"));
        }
        if row.link != skill.link {
            return Err(format!("skill {name} has the wrong link in {source}"));
        }
    }
    Ok(documented)
}

fn inventory_rows(text: &str) -> Result<Vec<String>, String> {
    let table = inventory_table(text)?;
    let mut table_lines = table.lines();
    let header = table_lines.next().ok_or("skill inventory table is missing")?;
    if !valid_header(&cells(header)?) {
        return Err("skill inventory header is invalid".into());
    }
    let separator = table_lines.next().ok_or("skill inventory separator is missing")?;
    if !separator.contains("---") {
        return Err("skill inventory separator is invalid".into());
    }
    let rows = table_lines.map(str::to_owned).collect::<Vec<_>>();
    if rows.is_empty() {
        return Err("skill inventory has no rows".into());
    }
    for row in &rows {
        if cells(row)?.len() != 3 {
            return Err(format!("skill inventory row has the wrong shape: {row}"));
        }
    }
    Ok(rows)
}

pub(super) fn inventory_row(text: &str, name: &str) -> Result<String, String> {
    inventory_rows(text)?
        .into_iter()
        .find(|row| row.contains(&format!("[{name}]")))
        .ok_or_else(|| format!("inventory row not found: {name}"))
}

fn row_name(row: &str) -> Result<String, String> {
    let cell = cells(row)?.into_iter().next().ok_or("skill name cell missing")?;
    let value = cell.strip_prefix('[').ok_or("skill name link is missing")?;
    value
        .split_once("](")
        .map(|(name, _)| name.to_owned())
        .filter(|name| !name.is_empty())
        .ok_or("skill name link is invalid".into())
}

fn row_value(row: &str) -> Result<DocumentedSkill, String> {
    let values = cells(row)?;
    let name_cell = values.first().ok_or("skill name cell missing")?;
    let link = name_cell
        .split_once("](")
        .and_then(|(_, target)| target.strip_suffix(')'))
        .filter(|target| !target.is_empty())
        .ok_or("skill link is invalid")?
        .to_owned();
    let purpose = values.get(1).ok_or("skill purpose cell missing")?.to_owned();
    if purpose.is_empty() {
        return Err("skill purpose is empty".into());
    }
    let component = values
        .get(2)
        .ok_or("skill component cell missing")?
        .trim_matches('`')
        .to_owned();
    if component.is_empty() {
        return Err("skill component is empty".into());
    }
    Ok(DocumentedSkill { purpose, component, link })
}

fn cells(row: &str) -> Result<Vec<String>, String> {
    let row = row.trim();
    if !row.starts_with('|') || !row.ends_with('|') {
        return Err(format!("not a complete Markdown table row: {row}"));
    }
    Ok(row
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_owned())
        .collect())
}

fn inventory_section(text: &str) -> Result<&str, String> {
    let (start, marker) = ["## Skill inventory", "## Skill 목록"]
        .iter()
        .find_map(|marker| text.find(marker).map(|start| (start, *marker)))
        .ok_or("missing skill inventory heading")?;
    let remainder = &text[start + marker.len()..];
    Ok(remainder.split("\n## ").next().unwrap_or(remainder))
}

pub(super) fn inventory_table(text: &str) -> Result<String, String> {
    let section = inventory_section(text)?;
    let mut lines = section.lines();
    let header = loop {
        let line = lines.next().ok_or("skill inventory table is missing")?;
        if line.trim_start().starts_with('|') && valid_header(&cells(line)?) {
            break line.to_owned();
        }
    };
    let separator = lines.next().ok_or("skill inventory separator is missing")?;
    let mut table = vec![header, separator.to_owned()];
    for line in lines {
        if !line.trim_start().starts_with('|') {
            break;
        }
        table.push(line.to_owned());
    }
    if table.len() == 2 {
        return Err("skill inventory has no rows".into());
    }
    Ok(table.join("\n"))
}

fn valid_header(cells: &[String]) -> bool {
    cells == ["Skill", "Purpose", "Component"] || cells == ["Skill", "목적", "컴포넌트"]
}

pub(super) fn guidance(text: &str, language: &str) -> Result<(), String> {
    for marker in [
        "$planning",
        "`planning`",
        "`orchestration`",
        "`engineering`",
        "plan-stress-test",
    ] {
        if !text.contains(marker) {
            return Err(format!("{language} README misses {marker} guidance"));
        }
    }
    let role_markers = if language == "English" {
        [
            "`planning` owns plan content",
            "`orchestration` owns execution coordination",
            "`engineering` owns individual implementation",
            "not an automatic review stage",
        ]
    } else {
        ["계획 내용", "실행 조정", "개별 구현", "자동 리뷰 단계가 아닙니다"]
    };
    for marker in role_markers {
        if !text.contains(marker) {
            return Err(format!("{language} README misses role distinction {marker}"));
        }
    }
    Ok(())
}
