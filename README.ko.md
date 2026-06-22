<p align="center">
  <img src="assets/codexy-agent-hero.png" alt="Codexy" width="100%">
</p>

<h1 align="center">Codexy</h1>

<p align="center">
  <a href="README.md">English</a>
</p>

<p align="center">
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-2f6f5e.svg"></a>
  <a href="https://github.com/eunsoogi/codexy/commits/main"><img alt="Last commit" src="https://img.shields.io/github/last-commit/eunsoogi/codexy.svg"></a>
  <a href="https://github.com/eunsoogi/codexy/issues"><img alt="GitHub issues" src="https://img.shields.io/github/issues/eunsoogi/codexy.svg"></a>
</p>

Codexy는 저장소 작업을 위한 Codex 하네스를 플러그인으로 패키지한
프로젝트입니다. 한 번의 프롬프트로 끝나지 않는 작업을 atomic lane으로
나누고, 적절한 worker/reviewer 표면에 연결하며, 증거를 남기고, GitHub
작업이 검증과 리뷰 게이트를 지나도록 돕습니다.

## 설치

Codexy는 설정된 Codex 플러그인 마켓플레이스 또는 로컬 마켓플레이스
항목에서 설치합니다. source-checkout 개발 환경에서는 사용자의 Codex
플러그인 마켓플레이스 설정에 맞게 이 저장소의 마켓플레이스 항목을
등록하거나 사용한 뒤, 그 마켓플레이스에서 Codexy를 설치합니다.

설치 후 Codex가 플러그인과 MCP 서버를 볼 수 있는지 확인합니다.

```sh
codex plugin list
codex mcp list
```

설치한 스킬, 전문 역할, MCP 도구가 현재 세션에 보이지 않으면 Codex를
재시작하거나 새 Codex 세션을 여세요.

## Codexy가 제공하는 것

Codexy는 이슈 단위 구현 lane, 격리된 worktree, 리뷰 응답 라우팅, 검증
증거, 그리고 current head가 실제로 리뷰되기 전에는 merge하지 않아야 하는
PR 흐름이 필요한 Codex 세션을 위한 플러그인입니다.

### 오케스트레이션과 Lane 제어

- **작업 전 분류**: Codexy는 setup, 편집, PR 처리, merge 작업 전에 lane
  유형, 소유자, 범위, 필요한 증거, 첫 허용 작업을 먼저 정하게 합니다. 큰
  요청이 하나의 뒤엉킨 브랜치로 변하는 것을 막기 위해서입니다.
- **이슈 단위 lane 분해**: 독립적인 결과물은 별도 브랜치, worktree, PR로
  나눕니다. 부모 오케스트레이션은 라우팅과 통합에 집중하고, child
  worktree thread가 구현을 소유합니다.
- **Goal과 plan 규율**: 장시간 작업은 보이는 goal/plan 상태를 유지합니다.
  child thread, 리뷰, 비동기 도구를 기다리는 시간이 흐릿한 "곧 완료" 상태로
  사라지지 않게 합니다.
- **부모/자식 소유권 경계**: Codexy는 실제 Codex worktree thread와 helper
  agent를 구분합니다. 브랜치와 PR이 있는 구현 작업은 owning child thread가
  맡고, 리뷰 피드백도 그 소유자에게 돌아갑니다.

### 전문 역할과 리뷰 게이트

- **목적별 전문 역할**: Codexy는 planning, architecture, implementation,
  refactoring, QA, release, workflow safety, repository mapping을 위한 역할을
  제공합니다. 단일 만능 assistant loop보다 더 분명한 역할 분담을 줍니다.
- **Sentinel readiness review**: 중요 lane은 PR-ready라고 말하기 전에 현재
  diff, 정확한 head, 범위, 검증 출력, 증거를 reviewer gate로 점검합니다.
- **소유권 혼동 없는 helper 역할**: 전문 agent는 lane 안에서 탐색, 리뷰,
  보조 작업을 할 수 있지만, 브랜치나 PR 또는 리뷰 응답 수정을 소유하는
  Codex worktree thread를 대체하지 않습니다.
- **리뷰 피드백 라우팅**: GitHub 또는 Codex 리뷰 코멘트가 child-owned PR에
  달리면 PR 번호, head SHA, 코멘트, 기대 증거, stop condition과 함께
  소유자에게 다시 라우팅합니다.

### 증거 표면: MCP, LSP, 저장소 탐색

- **Codegraph 탐색**: `codegraph` 표면은 편집 전 관련 파일, 의존 관계,
  주변 구현 표면을 찾게 돕습니다. 최종 판단은 직접 파일 readback으로 다시
  확인합니다.
- **언어 인식 체크**: `lsp` 표면은 해당 language server가 설정되어 있고
  실제로 사용 가능한지 기록합니다. 서버가 없거나 사용할 수 없으면
  diagnostics를 돌린 척하지 않고 그 사실을 handoff에 남깁니다.
- **도구 노출 증거**: Codexy는 "등록됨"과 "현재 세션에서 호출 가능함"을
  다른 사실로 취급합니다. 패키지된 도구가 기대되지만 사용할 수 없으면 그
  mismatch를 증거로 남깁니다.
- **저장소 네이티브 증거**: 로컬 명령, validator, PR 상태, 리뷰 thread,
  체크, 도구 출력처럼 다음 agent나 maintainer가 확인할 수 있는 표면에서
  증거를 모읍니다.

### Validator와 증거 기반 완료

- **플러그인 설정 검증**: validator는 manifest metadata, 마켓플레이스 등록,
  MCP/LSP 설정, 스킬, 전문 역할 metadata, 릴리스 contract를 점검합니다.
- **Completion handoff 체크**: PR이 열려 있거나, 리뷰 thread가 미해결이거나,
  리뷰 증거가 stale이거나, Codex 리뷰가 `eyes`로만 acknowledge된 상태에서
  완료를 주장하는 handoff를 거부할 수 있습니다.
- **Child-lane ownership 체크**: 잘못된 표면에 구현 소유권을 배정한 증거는
  PR-ready 전에 workflow defect로 처리합니다.
- **Touched-file 크기 체크**: 구현 파일과 test-harness 파일은 좁고 추적된
  예외가 없는 한 리뷰 가능한 크기로 유지합니다.
- **주장보다 증거 우선**: 테스트가 한 번 통과했다고 lane이 ready가 되지는
  않습니다. 증거는 현재 파일, 현재 커밋, 현재 PR head, 주장하는 외부 표면과
  맞아야 합니다.

### GitHub, PR, Merge 워크플로 지원

- **브랜치와 PR 규율**: 작업은 이슈 단위 범위에서 시작해 topic branch에
  담고, summary, rationale, changed areas, verification, not-run, follow-up,
  최종 issue link가 있는 구조화된 PR로 엽니다.
- **Current-head 리뷰 처리**: Codex 리뷰 요청은 PR head에 묶입니다. 새 커밋이
  올라오면 이전 리뷰 출력은 stale이며, `eyes` reaction은 리뷰 진행 중이라는
  acknowledgement일 뿐입니다.
- **리뷰 thread 정리**: actionable comment와 미해결 thread는 현재 head에서
  수정이 검증되고 resolve되거나 maintainer가 no-change로 수락하기 전까지
  merge blocker입니다.
- **Squash-merge 안전장치**: merge 흐름은 PR body를 보존하고, issue reference를
  검증하고, 리뷰된 head를 사용하고, 브랜치를 삭제하고, merge 후 main
  worktree를 검증합니다.
- **Post-merge 동기화**: merge 후 main을 갱신하고 merge 증거를 확인해야 lane을
  완료로 보고할 수 있습니다.

### 릴리스와 플러그인 패키징 지원

- **버전 동기화**: 릴리스 helper는 플러그인 metadata, 마켓플레이스 항목,
  package metadata가 일치하도록 돕습니다.
- **Runtime artifact 체크**: 패키징 검증은 runtime binary, platform support,
  생성된 plugin archive를 다룹니다.
- **Changelog 생성**: 릴리스 도구는 Git tag에서 changelog를 만들며, release
  history 밖의 더 새로운 tag를 잘못 기준으로 쓰지 않게 합니다.
- **마켓플레이스 publish contract**: validator는 source marketplace, package
  archive, workflow trigger, publish expectation을 릴리스 준비 증거로
  확인합니다.
- **로컬 설치 검증**: 릴리스 lane은 파일 생성만으로 준비됐다고 보지 않고,
  설치와 MCP visibility를 실제로 확인하는 증거를 포함합니다.
