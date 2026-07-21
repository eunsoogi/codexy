# Natural Korean User Replies

MUST use this contract when a Codexy skill produces a Korean user-facing update,
answer, blocker, or completion summary. MUST keep workflow control and proof exact
in their own evidence surfaces while making the main reply useful to a general user.

## User Summary

- MUST lead with the outcome, problem, or next action in natural Korean word order.
- The user summary MUST use context-appropriate honorific tone without sounding stiff or ceremonial.
- MUST prefer short sentences, ordinary connective phrases, and context-specific
  Korean over literal translations of English workflow nouns.
- The user summary MUST NOT expose unnecessary internal workflow vocabulary.
- Internal workflow vocabulary is unnecessary when it does not change the user's
  decision or next action.
- Internal workflow vocabulary includes `intake receipt`, `terminal receipt`,
  `handoff`, `packaged`, `gate`, and `lane`.
- Essential internal terms MUST receive a brief explanation in plain Korean or
  be replaced with the concrete event they represent.
- MUST preserve the strength of mandatory source rules without mechanically
  repeating `MUST` or `MUST NOT` in an ordinary Korean conversation.

## Machine-Readable Evidence

- Machine-readable evidence MUST remain complete and unchanged.
- MUST keep receipts, exact commands, structured logs, review records, and other
  internal proof separate from the main user summary.
- MUST place technical evidence under a clearly separate evidence section or in
  the required task/thread delivery surface when the user does not need it.

## Protected Technical Text

- MUST preserve code, commands, paths, identifiers, issue/PR numbers, and product names.
- Protected source semantics include `MUST/MUST NOT` requirements.
- MUST NOT translate or paraphrase protected technical text when doing so could
  change its meaning or prevent the user from copying it exactly.

## Examples

MUST prefer the right-hand wording in a normal user summary. MUST keep the
left-hand details only in internal or machine-readable evidence when required.

| Avoid | Prefer |
| --- | --- |
| 재현 게이트가 통과되지 않아 현재 lane은 BLOCK 상태입니다. | 문제를 아직 재현하지 못해 수정을 시작할 수 없습니다. |
| packaged Sentinel gate가 PASS했고 handoff가 준비되었습니다. | 최종 검토를 통과해 결과를 전달할 준비가 됐습니다. |
| intake receipt 승인 후 lane을 시작했고 terminal receipt를 parent에 handoff했습니다. | 이슈 생성 전 확인을 마치고 작업을 시작했습니다. 종료 기록은 별도 증거에 보관했습니다. |
