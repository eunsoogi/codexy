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

Codexy는 Codex 플러그인 마켓플레이스를 통해 설치합니다. 이 저장소가
아직 마켓플레이스 소스로 등록되어 있지 않다면 먼저 추가하세요.

```sh
codex plugin marketplace add eunsoogi/codexy --ref main
```

그다음 플러그인을 설치합니다.

```sh
codex plugin add codexy@codexy
```

Codex가 설치된 플러그인과 MCP 서버를 볼 수 있는지 확인합니다.

```sh
codex plugin list
codex mcp list
```

설치한 플러그인, 스킬, MCP 표면이 현재 세션에 보이지 않으면 Codex를
재시작하거나 새 Codex 세션을 여세요.

## Codexy가 제공하는 것

Codexy는 첫 프롬프트 이후에도 저장소 작업이 흐트러지지 않도록 돕는 Codex
하네스 플러그인입니다. 설치하면 작업 계획, 소유권 배정, 저장소 증거 수집,
리뷰 준비 상태 확인, 플러그인 릴리스 준비에 필요한 구체적인 Codex 표면을
추가합니다. 실제로는 issue에서 branch, PR, review, merge, release로 작업이
이동하는 동안 현재 소유자와 다음 단계에 필요한 증거를 잃지 않도록 Codex가
공유할 운영 모델을 제공합니다.

### 설치되는 하네스 표면

#### 워크플로 스킬

- **작업 분류**: agent가 편집을 시작하기 전에 lane type, owner, 필요한 증거,
  첫 허용 동작, 중단 조건을 먼저 이름 붙입니다.
- **Orchestration workflow**: parent session은 routing, status, merge decision을
  맡고 child worktree thread는 자기 branch 변경을 소유하게 합니다.
- **Git과 GitHub workflow**: issue intake, branch creation, PR body, label,
  review request, squash merge, branch cleanup, merge 후 동기화를 표준화합니다.
- **Proof-driven completion**: "완료"를 현재 파일 상태, branch head, PR 상태,
  check, review output, 외부 표면에 묶인 증거 checklist로 바꿉니다.
- **Release workflow**: version sync, package shape, marketplace metadata,
  archive check, release note, release handoff 작업을 안내합니다.

#### 저장소 도구 표면

- **Codegraph MCP**: 직접 파일을 읽고 patch하기 전에 관련 파일, symbol,
  dependency neighbor, validation touchpoint를 찾을 수 있는 repository graph
  표면을 제공합니다.
- **LSP MCP**: active workspace에서 language-aware diagnostic이 설정되어 있고
  실제 호출 가능한지 기록합니다. registered tool과 session에서 실제 callable한
  tool의 차이도 드러냅니다.
- **패키지된 MCP 등록**: session마다 MCP 설정을 손으로 다시 만들지 않고 같은
  설정을 검증할 수 있도록 plugin 안에 MCP 설정을 함께 제공합니다.

#### 전문 역할

- **Worker 역할**: implementation, refactoring, architecture, repository
  mapping, release preparation 등 focused lane에 쓸 수 있는 재사용 가능한 역할
  정의를 제공합니다.
- **Reviewer 역할**: regression, 누락된 verification, workflow rule 위반,
  readiness gap을 찾는 current-diff review prompt를 반복 가능하게 제공합니다.
- **Sentinel review gate**: non-trivial lane의 PR readiness 전에 사용할 packaged
  reviewer 기대치를 제공해, review evidence가 주장하는 정확한 branch나 diff에
  묶이게 합니다.

#### 검증과 릴리스 스크립트

- **Plugin configuration validator**: manifest metadata, marketplace
  registration, MCP entry, LSP catalog entry, skill frontmatter, agent
  definition, release metadata를 함께 확인합니다.
- **Workflow contract validator**: child-lane ownership claim, completion
  handoff, dirty-state exception, merge-message issue reference,
  review-readiness evidence를 검증합니다.
- **Version synchronization helper**: plugin version과 marketplace version을
  하나의 release surface로 확인하거나 갱신합니다.

### 작업 계획과 소유권 모델

#### 작업 접수

- **작업 분류**: 요청이 문서, 검증, 구현, 릴리스, 리뷰 대응, merge 중 어디에
  해당하는지 편집 전에 먼저 구분합니다.
- **범위 정리**: 큰 요청을 소유자, 수락 증거, 중단 조건이 있는 작은 lane으로
  나눕니다.
- **이슈 단위 실행**: 변경이 하나의 이슈, 하나의 branch, 하나의 리뷰 가능한
  PR로 이어지도록 돕습니다.

#### 스레드와 에이전트 경계

- **Parent session**: routing, 상태 확인, review thread 판단, merge readiness,
  merge 후 동기화를 조정합니다.
- **Child worktree thread**: 특정 branch와 issue-sized lane의 구현 및 review
  response patch를 소유합니다.
- **Specialist subagent**: focused analysis, 구현 조언, QA, current-diff review를
  보조하지만 branch owner가 되지는 않습니다.
- **소유권 증거**: 어느 표면이 branch를 소유하고 어느 표면이 보조했는지 남겨
  리뷰 피드백이 올바른 lane으로 돌아가게 합니다.

#### 긴 작업의 진행 관리

- **Goal tracking**: rebase, 리뷰 대기, 검증 실행, handoff 사이에서도 목표를
  계속 보이게 유지합니다.
- **Plan tracking**: lane을 pending, active, completed 단계로 나누어 현재 상태를
  드러냅니다.
- **Handoff discipline**: 다음 소유자에게 branch, head commit, 증거, blocker,
  중단 조건을 함께 전달하게 합니다.

### 검증과 리뷰 게이트

#### 저장소 Validator

- **플러그인 설정 검증**: manifest metadata, marketplace registration, MCP server
  entry, LSP catalog entry, skill, agent, release metadata를 확인합니다.
- **워크플로 계약 검증**: child-lane ownership claim, completion handoff,
  dirty-state exception, review-readiness claim, merge-message issue reference를
  검증합니다.
- **문서 변경 게이트**: 문서만 바뀌어도 whitespace, 파일 존재, touched surface
  확인을 거치게 합니다.

#### 리뷰 준비 상태

- **Current-head proof**: readiness를 오래된 diff의 리뷰가 아니라 현재 commit
  또는 PR head에 묶습니다.
- **Codex review gate**: 실질적인 Codex review evidence를 요구하고, `eyes`
  reaction은 merge approval이 아니라 review-in-progress 신호로 취급합니다.
- **Thread resolution check**: actionable하고 outdated가 아닌 review comment가
  수정, 수락, 설명되기 전까지 readiness 판단에 남깁니다.

#### GitHub 안전장치

- **구조화된 PR 흐름**: summary, rationale, verification evidence, follow-up,
  issue link를 PR에 남기도록 돕습니다.
- **Merge safeguard**: current-head match, squash merge, branch cleanup, merge 후
  `main` 동기화를 지원합니다.
- **중단 조건 보고**: PR을 merge할 수 없을 때 open PR을 완료로 보지 않고 정확한
  blocker를 보고하게 합니다.

### 플러그인 패키징과 릴리스 지원

#### 마켓플레이스 준비

- **Manifest 검증**: 공개 plugin identity, description, asset, runtime entry,
  install-facing metadata를 함께 확인합니다.
- **Marketplace 동기화**: marketplace registration이 패키지된 plugin version과
  metadata에 맞는지 확인합니다.
- **Asset check**: manifest가 참조하는 repository-level 및 plugin-local visual이
  존재하는지 검증합니다.

#### 릴리스 엔지니어링

- **버전 동기화**: plugin, package, marketplace version을 하나의 release
  surface로 확인하거나 갱신합니다.
- **Archive와 runtime check**: release handoff 전에 생성된 plugin archive와
  packaged MCP runtime을 검증합니다.
- **Release note 지원**: 의도한 Git history를 간결한 release note와 검증 증거로
  정리하도록 돕습니다.

#### 설치 검증

- **Plugin visibility check**: Codex가 설치된 plugin을 표시하는지 확인합니다.
- **MCP visibility check**: Codex가 설치된 MCP registration을 표시하는지
  확인합니다.
- **Fresh-session guidance**: 새로 설치한 plugin 표면이 active session에 보이지
  않을 때 restart 또는 새 session 확인을 명시합니다.
