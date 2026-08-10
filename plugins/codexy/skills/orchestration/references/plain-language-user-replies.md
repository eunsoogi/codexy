# Plain-Language User Replies

MUST use this contract for Codexy user-facing progress updates, blockers,
completion summaries, and next actions in English or Korean. MUST keep exact
workflow control and proof in their separate evidence surfaces.

## User Summary

- MUST lead with the outcome, problem, or next action.
- MUST replace unnecessary internal workflow terms with the concrete event they
  represent.
- An essential internal term MUST receive a brief adjacent explanation when the
  exact term materially affects the user's decision or next action.
- MUST NOT expose an unexplained internal term merely because it appears in the
  source contract.
- MUST NOT weaken or omit the underlying requirement when simplifying the
  summary.
- MUST keep next-action claims faithful to verified evidence.

## English

- MUST prefer short, direct sentences and ordinary workflow language.
- MUST describe what changed, what is waiting, or what the user should do rather
  than naming the internal orchestration mechanism.

| Internal term | Prefer in an English user summary |
| --- | --- |
| `Sentinel verdict` | final review result |
| `terminal handoff` | final status and next action |
| `delta` | changed fact |
| `heartbeat route` | scheduled read-only check |
| `gate` | required check |
| `lane` | this issue |
| `packaged` | bundled with Codexy |
| `faithful RED coverage` | original-failure test |

## Korean

- MUST use natural Korean word order, context-appropriate honorific tone, short
  sentences, and ordinary connective phrases.
- MUST translate the concrete event, not the English workflow noun.

| Internal term | Prefer in a Korean user summary |
| --- | --- |
| `Sentinel verdict` | 최종 검토 결과 |
| `terminal handoff` | 최종 상태와 다음 조치 |
| `delta` | 달라진 사실 |
| `heartbeat route` | 예약된 읽기 전용 점검 |
| `gate` | 필수 확인 |
| `lane` | 이 이슈 |
| `packaged` | Codexy에 포함된 |
| `faithful RED coverage` | 원래 실패를 보여 주는 테스트 |

## Protected Evidence

- Exact schema names, validator fields, commands, identifiers, and
  machine-readable evidence MUST remain complete and unchanged.
- Code, paths, issue/PR numbers, product names, structured receipt fields, and
  `MUST/MUST NOT` semantics MUST remain exact when they are evidence or
  copyable technical text.
- MUST keep protected evidence separate from the user summary. This boundary
  changes presentation only; it MUST NOT rename internal contracts.

## Examples

| Avoid | Prefer |
| --- | --- |
| Sentinel verdict: PASS. The terminal handoff is ready. | The final review passed, so the result is ready to share. |
| The heartbeat route is waiting on the final gate. | A scheduled read-only check is waiting for the final required check. |
| This lane has faithful RED coverage. | This issue now has a test that demonstrates the failure before the fix. |
| packaged Sentinel gate가 PASS했고 terminal handoff가 준비되었습니다. | 최종 검토를 통과해 결과를 전달할 준비가 됐습니다. |
| heartbeat route가 마지막 gate를 기다리고 있습니다. | 예약된 읽기 전용 점검이 마지막 필수 확인을 기다리고 있습니다. |
| 이 lane에 faithful RED coverage를 추가했습니다. | 이 이슈에 수정 전 실패를 보여 주는 테스트를 추가했습니다. |
