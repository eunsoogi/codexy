# Model migration and verification

Codexy's source configuration selects GPT-6.2 for the existing Astra and Sol
roles. It does not change their responsibilities, automatically migrate active
sessions, or establish that a particular client or account can execute a model.
A source merge is not a release, installed update, or effective-model receipt.

## Configured routes

| Role                    | Model           | Effort   |
| ----------------------- | --------------- | -------- |
| Parent / Orchestrator   | `gpt-6.2-sol`   | `low`    |
| Ordinary Worker         | `gpt-6.1-terra` | `medium` |
| Evidenced hard Worker   | `gpt-6.2-sol`   | `medium` |
| Architect and Warden    | `gpt-6.2-astra` | `high`   |
| Advisor                 | `gpt-6.2-astra` | `medium` |
| Sentinel and Shipwright | `gpt-6.2-sol`   | `high`   |
| Optional GitHub Weaver  | `gpt-6.2-sol`   | `medium` |
| Auditor                 | `gpt-6.1-terra` | `medium` |
| Cartographer            | `gpt-6-luna`    | `low`    |

The specialist TOMLs are the configuration source of truth. Ordinary Workers
remain on Terra; hard-Worker routing still requires qualifying work facts.
Sentinel remains an optional final reviewer rather than a mandatory intermediate
stage. None of these changes adds service-tier, pricing, or context-window
settings.

## Model replacement and user configuration

The targeted Astra/Sol model identifiers are replaced in the existing routes.
There is no old-generation model allowlist, fallback, or compatibility layer.
Creation and recipient-bound delivery use the current supported pairs; an
unsupported pair is rejected rather than silently upgraded or accepted.
Recipient identity and capability checks remain unchanged.

Packaged specialist definitions and their managed projection are distinct from
user-owned configuration. An authorized update uses the existing registration
and ownership checks; it is not a global search-and-replace of personal
configuration or session files. Preserve user-owned entries and report a
collision rather than overwriting them. Review the
[component installation contract](getcodexy-component-installation.md) before an
installed update. Updating configuration does not change an already-running
session's actual model.

Historical evidence, explicit rejection fixtures, and unrelated model families
retain their original names. Do not rewrite a previous run to appear to have
used a newer model.

## Support and evidence boundaries

[Official Codex model guidance](https://learn.chatgpt.com/docs/models?surface=cli)
and the
[GPT-6.2 rollout documentation](https://learn.chatgpt.com/docs/whats-new/gpt-62-astra-sol)
identify the model family and supported client surfaces. Availability remains
subject to the actual client, workspace, and account. Codex app Light maps to
CLI Low; retained effort labels are not a claim of equal work or performance
across model generations. API-only settings are not Codex-host guarantees.

Keep these observations separate:

| Layer                | What it establishes                                     | What it does not establish              |
| -------------------- | ------------------------------------------------------- | --------------------------------------- |
| Source configuration | Packaged TOML and routing-policy values                 | Installed or executed settings          |
| Generated projection | Managed files produced in the inspected target          | Availability or actual execution        |
| Requested parameters | Model and effort submitted for the bound recipient      | Accepted or effective model             |
| Host readback        | Values explicitly reported for that session by the host | Unreported values or unrelated sessions |

Before executing an installed candidate, inspect the target client's available
models and effort controls for the account. If a pair is unsupported, stop that
execution and record the exact unsupported pair and client; do not silently fall
back. If effective model or effort telemetry is unavailable, record it as
unknown even when the request succeeded. A unit-test fixture, printed model
name, or this table cannot substitute for host readback.

## Proportionate acceptance

After routing and specialist changes are integrated, verify the parent request,
ordinary and hard Worker creation, authenticated recipient-bound continuation,
specialist registration, and optional-final-review selection. Include
replaced-model rejection, user-owned configuration preservation, and
managed-projection updates in regression coverage. Run the relevant plugin,
hook, runtime, and registration tests against the same integrated source.

For a separately authorized installed check, record the candidate revision and
client version, inspect configuration without exposing private contents, then
exercise a new target-model session. Capture requested and host-observed
settings separately. Never mutate a user's active session simply to manufacture
migration evidence.

Use a small set of familiar tasks to compare correctness and handoff integrity.
Record elapsed time and usage or cost only when actually observable, with the
same task and environment noted. Source tests validate routing and preservation,
not comparative model quality, latency, or price. Missing live observations
remain explicit verification gaps; they do not justify invented benchmark
results or an exhaustive evaluation gate.
