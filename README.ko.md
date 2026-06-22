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

Codexy는 Codex를 단발성 코딩 세션이 아니라 긴 저장소 작업을 다루는
하네스로 확장합니다. 스킬, 전문 역할, MCP 서버, validator, 릴리스 helper를
함께 패키지해 agent가 계획, 구현, 검증, 리뷰, merge를 ownership과 증거를
잃지 않고 이어가도록 돕습니다.

### 워크플로 제어

#### 범위 설정

- **작업 분류**: lane 종류, 소유자, 범위, 필요한 증거를 먼저 정합니다.
- **Atomic lane**: 큰 요청을 이슈 단위 branch, worktree, PR로 나눌 수 있게
  돕습니다.

#### 소유권

- **서브스레드**: 특정 branch나 PR의 구현을 소유하는 Codex worktree thread를
  지원합니다.
- **서브에이전트**: helper/reviewer agent를 지원하되, branch-owning 구현
  thread와 혼동하지 않게 합니다.
- **Parent orchestration**: routing, review follow-up, merge coordination을
  child-owned patch work와 분리합니다.

#### 진행 상태

- **Goal과 plan discipline**: 리뷰 대기, 검증, handoff가 있는 긴 작업의 상태를
  보이게 유지합니다.
- **리뷰 라우팅**: 피드백을 변경을 소유한 lane으로 되돌려 보냅니다.

### Agent와 Tooling 표면

#### 스킬

- **워크플로 스킬**: task classification, orchestration, Git/GitHub flow,
  proof-driven completion, QA, release engineering, refactoring, debugging,
  test-driven development를 제공합니다.
- **설치된 지침**: 새 Codex 세션도 같은 워크플로 지침을 따를 수 있게 스킬을
  플러그인 안에 포함합니다.

#### 전문 Agent

- **Worker 역할**: architecture, implementation, refactoring, repository
  mapping, release work를 위한 agent definition을 제공합니다.
- **Reviewer 역할**: current diff readiness를 확인하는 sentinel reviewer를
  제공합니다.

#### MCP와 LSP 통합

- **Codegraph**: 코드 변경 전 관련 파일과 의존성을 찾습니다.
- **LSP**: language-aware edit에 필요한 language server 등록과 사용 가능성을
  확인합니다.
- **도구 노출 체크**: configured, registered, callable 상태를 구분합니다.

### 검증과 리뷰 게이트

#### Validator

- **플러그인 설정 체크**: plugin manifest metadata, marketplace entry, MCP
  registration, LSP catalog, skill, agent, release metadata를 검증합니다.
- **워크플로 증거 체크**: completion handoff, child-lane ownership evidence,
  review state, merge-message issue reference를 검증합니다.

#### 리뷰 준비 상태

- **Current-head evidence**: readiness를 현재 파일, 커밋, PR head에 묶습니다.
- **Codex review gate**: 실제 Codex 리뷰 출력을 요구하고, `eyes` reaction은
  진행 중 신호로만 봅니다.
- **Review thread 처리**: actionable comment가 수정되거나 명시적으로 수락될
  때까지 readiness 판단에 남깁니다.

#### GitHub와 Merge 안전장치

- **구조화된 PR**: summary, rationale, verification, follow-up, issue link를
  갖춘 PR을 지향합니다.
- **Merge discipline**: squash merge, branch cleanup, merge 후 `main` 갱신을
  지원합니다.

### 릴리스와 플러그인 패키징

#### 마켓플레이스 준비

- **Manifest와 asset 체크**: plugin metadata, marketplace registration, packaged
  asset을 함께 검증합니다.
- **버전 동기화**: plugin, marketplace, package metadata를 맞춥니다.
- **Runtime artifact**: 생성된 archive와 packaged MCP runtime을 확인합니다.

#### 릴리스 워크플로 지원

- **Changelog helper**: 의도한 Git history에서 release note를 만듭니다.
- **설치 검증**: release된 plugin이 설치되고 Codex에서 MCP 표면이 보이는지
  확인합니다.
