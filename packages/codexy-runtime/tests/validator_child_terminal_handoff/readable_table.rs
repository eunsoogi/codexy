use super::TestResult;

#[test]
fn readable_wait_producer_preserves_pending_and_rejects_actionable_state() -> TestResult {
    let evidence = "Lane ownership: child-owned\nNonterminal wait handoff:\n| Field | Value |\n| --- | --- |\n| state fingerprint | current |\n| producer state | reviewer is running |\n| wake route | review result |\n| ownership | retained |\n| goal state | active |\n| plan state | active |\n| goal transition | none |\n| return control | confirmed |\n";
    let output = super::run_validator(evidence)?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for state in [
        "reviewer is not running",
        "review feedback requires changes",
        "reviewer is running with unresolved review feedback",
    ] {
        assert!(
            !super::run_validator(&evidence.replace("reviewer is running", state))?
                .status
                .success()
        );
    }
    Ok(())
}

#[test]
fn phase_table_rejects_unfinished_goal_unauthorized_delivery_and_replay() -> TestResult {
    let evidence = format!(
        "{}Idle child state: goal state=complete; plan state=idle\n",
        super::records::phase_evidence(
            "delivery=confirmed; task surface=codex task/thread",
            "complete"
        )
    );
    let readable = evidence
        .lines()
        .map(|line| {
            if let Some((title, fields)) = line.split_once(": ").filter(|(title, _)| {
                ["Idle wait handoff", "Goal tool result", "Idle child state"].contains(title)
            }) {
                let mut table = format!("{title}:\n| Field | Value |\n| --- | --- |\n");
                for field in fields.split(';') {
                    if let Some((key, value)) = field.trim().split_once('=') {
                        table.push_str(&format!(
                            "| {} | {} |\n",
                            key.trim(),
                            value.trim().replace('|', "\\|")
                        ));
                    }
                }
                table
            } else {
                format!("{line}\n")
            }
        })
        .collect::<String>();
    assert!(super::run_validator(&readable)?.status.success());
    for invalid in [
        readable.replace(
            "| phase criteria | satisfied |",
            "| phase criteria | unfinished |",
        ),
        readable.replace(
            "| delivery | confirmed |",
            "| delivery | confirmed |\n| message permission | absent |",
        ),
        format!("{readable}{readable}"),
    ] {
        assert!(
            !super::run_validator(&invalid)?.status.success(),
            "{invalid}"
        );
    }
    Ok(())
}

#[test]
fn terminal_handoff_accepts_a_readable_table() -> TestResult {
    let evidence = "Lane ownership: child-owned\nSource thread id: parent\nTerminal parent handoff:\n| Field | Value |\n| --- | --- |\n| Event id | archive-event |\n| Issue/PR | #1 / PR #2 |\n| Child task | child |\n| Parent task | parent |\n| Branch | topic |\n| Worktree | /worktree |\n| Head | abc |\n| Clean/index | clean |\n| Last proof | validator passed |\n| Current gate | parent review |\n| Preserved reservation/artifacts | worktree |\n| Parent next action | review |\n| Delivery | confirmed |\n| Task surface | codex task/thread |\nTerminal child transition: action=archive\n";
    let output = super::run_validator(evidence)?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let reordered = evidence
        .replace("| Event id | archive-event |\n", "")
        .replace(
            "| Issue/PR |",
            "| Event id | archive\\|event |\n| Issue/PR |",
        )
        .replace(
            "validator passed",
            "[validator](https://example.org/check); passed",
        );
    assert!(super::run_validator(&reordered)?.status.success());
    let parent_table = evidence.replace(
        "Source thread id: parent",
        "Source thread id:\n| Field | Value |\n| --- | --- |\n| Parent task | parent |",
    );
    assert!(super::run_validator(&parent_table)?.status.success());
    assert!(
        !super::run_validator(&parent_table.replacen(
            "| Parent task | parent |",
            "| Parent task | wrong |",
            1
        ))?
        .status
        .success()
    );
    assert!(
        !super::run_validator(&parent_table.replacen("| Parent task | parent |\n", "", 1))?
            .status
            .success()
    );
    let multi_task = format!(
        "| Task | Status | Next owner |\n| --- | --- | --- |\n| child-a | running | worker-a |\n| child-b | waiting | parent |\n\n{evidence}"
    );
    assert!(super::run_validator(&multi_task)?.status.success());
    let historical = evidence
        .replace(
            "Terminal parent handoff:\n",
            "~~~markdown\nTerminal parent handoff:\n",
        )
        .replace(
            "Terminal child transition: action=archive",
            "~~~\nTerminal child transition: action=archive",
        );
    assert!(!super::run_validator(&historical)?.status.success());
    let formatted = evidence
        .replace("Terminal parent handoff:\n", "Parent handoff:\n\n")
        .replace("| Field | Value |", "| Item | Details |")
        .replace("| Parent task | parent |", "| Parent | `parent` |")
        .replace("| Head | abc |", "| Head SHA | abc |")
        .replace("| Last proof |", "| Verification |")
        .replace("validator passed |", "validator\npassed |")
        .replace(
            "| Child task | child |",
            "| Child task | [child](https://example.org/task) |",
        );
    assert!(super::run_validator(&formatted)?.status.success());
    for invalid in [
        evidence.replace(
            "| Parent task | parent |",
            "| Parent task | somebody-else |",
        ),
        evidence.replace(
            "Source thread id: parent\n",
            "Source thread id: parent\nSource thread id: other-parent\n",
        ),
        evidence.replace(
            "| Delivery | confirmed |",
            "| Delivery | confirmed |\n| Delivery | unconfirmed |",
        ),
        evidence.replace("| Head | abc |\n", ""),
        evidence.replace("| Head | abc |", "| Head | abc |\n| Head SHA | other |"),
        evidence
            .replace("Terminal parent handoff:", "> Terminal parent handoff:")
            .replace("| Field", "> | Field")
            .replace("| ---", "> | ---")
            .replace("| Event", "> | Event")
            .replace("| Issue", "> | Issue")
            .replace("| Child", "> | Child")
            .replace("| Parent", "> | Parent")
            .replace("| Branch", "> | Branch")
            .replace("| Worktree", "> | Worktree")
            .replace("| Head", "> | Head")
            .replace("| Clean", "> | Clean")
            .replace("| Last", "> | Last")
            .replace("| Current", "> | Current")
            .replace("| Preserved", "> | Preserved")
            .replace("| Delivery", "> | Delivery")
            .replace("| Task", "> | Task"),
    ] {
        assert!(
            !super::run_validator(&invalid)?.status.success(),
            "{invalid}"
        );
    }
    Ok(())
}

#[test]
fn blocked_audit_accepts_tables_and_rejects_new_parent_direction() -> TestResult {
    let evidence = format!(
        "Lane ownership: child-owned\nSource thread id: parent-375\nGoal control state: source_thread_id=parent-375\n{}",
        super::records::terminal_goal_transition("blocked", "readable", true)
    );
    let readable = evidence
        .lines()
        .map(|line| {
            let Some((title, fields)) = line.split_once(": ") else {
                return format!("{line}\n");
            };
            if ![
                "Blocked goal user-decision gate",
                "Parent goal pre-delivery",
                "Blocked goal pre-mutation check",
                "Terminal parent handoff",
                "Parent goal post-result",
            ]
            .contains(&title)
            {
                return format!("{line}\n");
            }
            let mut table = format!("{title}:\n| Field | Value |\n| --- | --- |\n");
            for field in fields.split(';') {
                if let Some((key, value)) = field.trim().split_once('=') {
                    table.push_str(&format!(
                        "| {} | {} |\n",
                        key.trim(),
                        value.trim().replace('|', "\\|")
                    ));
                }
            }
            table
        })
        .collect::<String>();
    let accepted = super::run_validator(&readable)?;
    assert!(
        accepted.status.success(),
        "{}",
        String::from_utf8_lossy(&accepted.stderr)
    );
    let cancelled = readable.replace(
        "Goal tool call: update_goal",
        "Parent direction event: version=new; cancellation=present\nGoal tool call: update_goal",
    );
    assert!(!super::run_validator(&cancelled)?.status.success());
    Ok(())
}
