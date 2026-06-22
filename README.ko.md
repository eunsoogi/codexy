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
하네스 플러그인입니다. 워크플로 지침, 전문 역할 정의, MCP 서버 등록,
validator, 릴리스 유틸리티를 함께 제공해 여러 단계의 코딩 작업을 라우팅,
검증, 리뷰, 완료까지 이어갈 수 있게 합니다.

### 작업 계획과 소유권

#### 작업 접수

- **작업 분류**: 요청이 문서, 검증, 구현, 릴리스, 리뷰 대응, merge 중 어디에
  해당하는지 편집 전에 먼저 구분합니다.
- **범위 정리**: 큰 요청을 소유자, 수락 증거, 중단 조건이 있는 작은 lane으로
  나눕니다.
- **이슈 단위 실행**: 변경이 하나의 이슈, 하나의 branch, 하나의 리뷰 가능한
  PR로 이어지도록 돕습니다.

#### 스레드와 에이전트 경계

- **Parent orchestration**: 조정 세션은 routing, 상태 확인, review thread 판단,
  merge readiness를 맡습니다.
- **Child worktree thread**: 별도 worktree와 branch를 가진 Codex thread를 해당
  lane의 구현 소유자로 취급합니다.
- **Specialist subagent**: helper나 reviewer agent는 분석, 조언, current-diff
  review에 쓰되 branch를 소유한 child thread와 혼동하지 않게 합니다.
- **소유권 증거**: 어느 표면이 branch를 소유하고 어느 표면이 보조했는지 남겨
  리뷰 피드백이 올바른 lane으로 돌아가게 합니다.

#### 긴 작업의 진행 관리

- **Goal tracking**: rebase, 리뷰 대기, 검증 실행, handoff 사이에서도 목표를
  계속 보이게 유지합니다.
- **Plan tracking**: lane을 pending, active, completed 단계로 나누어 현재 상태를
  드러냅니다.
- **Handoff discipline**: 다음 소유자에게 branch, head commit, 증거, blocker,
  중단 조건을 함께 전달하게 합니다.

### 저장소 이해와 도구 표면

#### 코드 탐색

- **Codegraph 등록**: 코드 수정 전에 관련 파일, symbol, 의존성, validator
  touchpoint를 찾을 수 있는 저장소 graph 표면을 제공합니다.
- **직접 readback 기대치**: graph 결과는 탐색 증거로 쓰고, 편집 전에는 실제
  파일을 다시 읽도록 요구합니다.

#### 언어 인식 작업

- **LSP 등록**: active session에서 language-aware diagnostic을 쓸 수 있는지
  확인할 수 있도록 language server 설정을 패키지합니다.
- **노출 상태 확인**: configured, registered, callable 상태를 분리해 plugin 또는
  session setup 문제를 드러냅니다.

#### 전문 역할 카탈로그

- **Worker 역할**: 구현, architecture, refactoring, repository mapping, release
  preparation에 맞춘 역할 정의를 제공합니다.
- **Reviewer 역할**: regression, 누락된 검증, workflow rule 위반을 찾는
  current-diff review 역할을 제공합니다.
- **역할 일관성**: 세션마다 즉석에서 역할을 만들지 않고 같은 prompt와 경계를
  반복 사용할 수 있게 합니다.

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
