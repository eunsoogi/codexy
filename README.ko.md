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

현재 저장소 마켓플레이스 항목은 다음 명령으로 등록합니다.

```sh
codex plugin marketplace add eunsoogi/codexy --ref main
```

그다음 해당 마켓플레이스에서 플러그인을 설치합니다.

```sh
codex plugin add codexy@codexy
```

설치 후 Codex가 플러그인과 MCP 서버를 볼 수 있는지 확인합니다.

```sh
codex plugin list
codex mcp list
```

설치한 플러그인, 스킬, MCP 표면이 현재 세션에 보이지 않으면 Codex를
재시작하거나 새 Codex 세션을 여세요.

## Codexy가 제공하는 것

Codexy는 범위가 잡힌 구현 lane, 격리된 worktree, 리뷰 응답 라우팅, 검증
증거, 여러 agent turn 이후에도 이해 가능한 PR 흐름이 필요한 Codex 세션을
위한 플러그인입니다.

### 오케스트레이션과 Lane 제어

- **작업 분류**: Codexy는 작업 종류, 소유자, 범위, 필요한 증거를 먼저
  정하게 해 작업이 파일과 브랜치 전반으로 번지는 것을 줄입니다.
- **이슈 단위 lane**: 큰 요청을 집중된 브랜치, worktree, PR로 나눠 서로 다른
  결과물이 한 리뷰에 섞이지 않게 합니다.
- **Goal과 plan 상태**: 장시간 작업에서 handoff, 리뷰 대기, 검증, 후속 작업의
  진행 상태를 보이게 유지합니다.
- **부모/자식 경계**: 오케스트레이션과 구현 소유권을 분리해 누가 패치하고,
  검증하고, 리뷰 피드백에 응답해야 하는지 더 분명하게 만듭니다.

### 전문 역할과 리뷰 게이트

- **목적별 역할**: Codexy는 planning, architecture, implementation,
  refactoring, QA, release, workflow safety, repository mapping을 위한 역할을
  패키지합니다.
- **Readiness review**: sentinel 방식 리뷰는 handoff나 PR-ready 주장 전에
  범위, 증거, 검증을 한 번 더 살피는 역할을 합니다.
- **명확한 helper 의미**: 전문 역할은 lane 안의 탐색, 리뷰, 보조 작업으로
  다뤄지고, 브랜치와 PR로 보이는 구현 작업은 worktree 기반 흐름에 묶입니다.
- **리뷰 응답 지원**: 리뷰 코멘트는 해당 변경을 소유한 lane으로 되돌아가
  수정과 후속 증거의 맥락을 유지할 수 있습니다.

### 증거 표면: MCP, LSP, 저장소 탐색

- **Codegraph 탐색**: `codegraph`는 편집 전 관련 파일, 의존 관계, 주변 표면을
  찾는 데 도움을 줍니다.
- **언어 인식 체크**: `lsp`는 검토 중인 파일에 맞는 language server가 설정되어
  있고 실제로 사용할 수 있는지 기록합니다.
- **도구 사용 가능성 증거**: Codexy는 설정된 도구와 현재 세션에서 실제로
  호출 가능한 도구를 구분합니다.
- **저장소 네이티브 증거**: 명령 출력, validator 결과, PR 상태, 리뷰 thread,
  체크, 도구 출력은 다음 agent나 maintainer가 확인할 수 있는 증거가 됩니다.

### Validator와 증거 기반 완료

- **플러그인 설정 검증**: validator는 manifest metadata, 마켓플레이스 등록,
  MCP/LSP 설정, 스킬, 역할 metadata, 릴리스 contract를 다룹니다.
- **완료 증거 체크**: handoff 증거를 PR 상태, 리뷰 상태, current-head 리뷰
  출력과 비교해 ready 주장에 필요한 증거를 확인할 수 있습니다.
- **소유권 증거 체크**: child-lane 증거는 오케스트레이션, helper 작업,
  브랜치 소유 구현이 섞이는 혼동을 잡는 데 도움을 줍니다.
- **리뷰 가능한 파일 크기**: 변경된 구현 파일과 test-harness 파일이 로컬 크기
  기준을 넘는지 확인해 리뷰 부담을 줄입니다.
- **현재 상태 기반 증거**: Codexy는 현재 파일, 커밋, PR head, 논의 중인 외부
  표면과 맞는 증거를 중시합니다.

### GitHub, PR, Merge 워크플로 지원

- **구조화된 PR**: Codexy는 summary, rationale, changed areas, verification,
  not-run, follow-up, issue link가 있는 PR을 만들도록 돕습니다.
- **Current-head 리뷰 인식**: 리뷰 증거가 어떤 PR head를 검토했는지 드러나
  새 커밋 이후 stale feedback을 알아보기 쉽습니다.
- **리뷰 thread 가시성**: actionable comment와 미해결 thread가 readiness
  판단의 일부로 남습니다.
- **Squash-merge 지원**: PR body 맥락 보존, issue reference, branch cleanup,
  post-merge 검증을 중심으로 merge 증거를 정리합니다.
- **Post-merge 증거**: 갱신된 main 상태와 merge-message 체크는 저장소가 PR이
  말한 상태로 끝났는지 확인하는 데 도움을 줍니다.

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
