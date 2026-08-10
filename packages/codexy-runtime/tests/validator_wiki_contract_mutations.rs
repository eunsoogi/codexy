type TestResult = Result<(), Box<dyn std::error::Error>>;

use crate::support::wiki_core_contract::validate_core_skill;
use crate::support::wiki_migration_rules::validate_migration_rules;

const REMOVED_WORKFLOWS: &[&str] = &[
    "collect", "plan", "project", "inventory", "dataset", "archive", "ll", "status", "session",
    "session-capture", "rehydrate", "feedback", "feedback-capture",
];

#[test]
fn parsed_skill_shape_rejects_each_core_identity_mutation() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let skill = std::fs::read_to_string(root.join("plugins/codexy/skills/wiki/SKILL.md"))?;
    for (name, mutation) in [
        ("heading", skill.replacen("## Core workflow", "## Retired workflow", 1)),
        ("inventory", skill.replacen("init → ingest → compile → query → refresh", "init → query", 1)),
        ("link", skill.replacen("[Migration](references/migration.md)", "Migration", 1)),
        ("removed", format!("{skill}\n`collect`")),
    ] {
        assert!(validate_core_skill(&mutation, REMOVED_WORKFLOWS).is_err(), "{name}");
    }
    Ok(())
}

#[test]
fn normalized_migration_rules_reject_each_required_rule_mutation() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let guide = std::fs::read_to_string(root.join("plugins/codexy/skills/wiki/references/migration.md"))?;
    for (name, required) in [
        ("preservation", "MUST preserve existing `raw/`, `wiki/`, `_index.md`, and `log.md`"),
        ("no deletion", "MUST NOT delete, overwrite, or rename existing topic data"),
        ("source scalar", "`sources:` scalar"),
        ("provenance gap", "provenance gap"),
        ("preflight", "MUST validate every referenced provenance and freshness input before any log"),
        ("write", "MUST append one migration entry"),
    ] {
        let mutation = guide.replacen(required, "retired rule", 1);
        assert!(validate_migration_rules(&mutation).is_err(), "{name}");
    }
    let source_key = guide.replacen("`sources:`", "`origin:`", 1);
    assert!(validate_migration_rules(&source_key).is_err(), "inline source key");
    for identity in ["raw/", "wiki/", "_index.md", "log.md"] {
        let mutation = guide.replacen(identity, "retired", 1);
        assert!(validate_migration_rules(&mutation).is_err(), "inline {identity}");
    }
    let rule = "MUST validate every referenced provenance and freshness input before any log\n   or derived write";
    for (name, mutation) in [
        ("comment", format!("<!-- {rule} -->")),
        ("fence", format!("```text\n{rule}\n```")),
        ("inline", format!("`{rule}`")),
        ("html", format!("<div>{rule}</div>")),
        ("negated", rule.replacen("MUST validate", "MUST NOT validate", 1)),
        ("weakened", rule.replacen("MUST validate", "MAY validate", 1)),
        ("duplicate", format!("{rule}. {rule}")),
        ("conflict", format!("{rule}. MUST NOT validate every referenced provenance and freshness input before any log or derived write")),
        ("except", format!("{rule} except one route")),
        ("unless", format!("{rule} unless a route exists")),
    ] {
        let mutated = guide.replacen(rule, &mutation, 1);
        assert!(validate_migration_rules(&mutated).is_err(), "{name}");
    }
    for qualifier in ["baseline", "allowlist", "compatibility", "alias", "external", "restore"] {
        let mutated = guide.replacen(rule, &format!("{qualifier} route: {rule}"), 1);
        assert!(validate_migration_rules(&mutated).is_err(), "{qualifier}");
    }
    let article = "---\ntitle: Test\n---\nbody";
    assert!(crate::support::wiki_core_contract::frontmatter_string(article, "title").is_ok());
    let bad_closing = article.replacen("\n---\nbody", "\n---garbage\nbody", 1);
    assert!(crate::support::wiki_core_contract::frontmatter_string(&bad_closing, "title").is_err());
    Ok(())
}

#[test]
fn migration_rules_bind_exact_inline_identities_and_clause_local_qualifiers() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let guide = std::fs::read_to_string(root.join("plugins/codexy/skills/wiki/references/migration.md"))?;
    for (name, from, to) in [
        ("raw decoy", "`raw/`", "`raw`"),
        ("wiki decoy", "`wiki/`", "`wiki-old`"),
        ("index decoy", "`_index.md`", "`_index.md.bak`"),
        ("log decoy", "`log.md`", "`log.txt`"),
    ] {
        assert!(validate_migration_rules(&guide.replacen(from, to, 1)).is_err(), "{name}");
    }
    let moved_source = guide.replacen("`sources:` scalar", "`origin:` scalar", 1);
    let moved_source = format!("{moved_source}\n4. Explanation: `sources:` is a YAML key.");
    assert!(validate_migration_rules(&moved_source).is_err(), "moved sources identity");
    let unrelated = guide.replacen(
        "Unsupported material remains untouched and is not evidence for compiled articles.",
        "Unsupported material remains untouched and is not evidence for compiled articles. External explanation only.",
        1,
    );
    let unrelated_result = validate_migration_rules(&unrelated);
    assert!(unrelated_result.is_ok(), "unrelated qualifier prose: {unrelated_result:?}");
    Ok(())
}

#[test]
fn migration_rules_reject_active_token_stream_polarity_and_prefix_mutations() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let guide = std::fs::read_to_string(root.join("plugins/codexy/skills/wiki/references/migration.md"))?;
    let rule = "MUST validate every referenced provenance and freshness input before any log\n   or derived write";
    for (name, replacement) in [
        ("multiple spaces", "MUST  NOT validate"),
        ("tab", "MUST\tNOT validate"),
        ("soft break", "MUST\nNOT validate"),
        ("split event", "MUST **NOT** validate"),
    ] {
        let mutation = guide.replacen(rule, &rule.replacen("MUST validate", replacement, 1), 1);
        assert!(validate_migration_rules(&mutation).is_err(), "{name}");
    }
    let prefix = rule.replacen("MUST validate", "Unless `route`, MUST validate", 1);
    assert!(validate_migration_rules(&guide.replacen(rule, &prefix, 1)).is_err(), "split prefix");
    Ok(())
}
