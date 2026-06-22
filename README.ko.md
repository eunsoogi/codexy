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

Codexy는 Codex를 위한 하네스와 루프 엔지니어링 프로젝트입니다. 에이전트 작업을 가볍고 저장소 친화적인 시스템으로 구조화하고, 실행하고, 관찰하고, 검증할 수 있게 돕습니다.

## 포함된 것

Codexy는 Codex 하네스를 플러그인 형태로 패키지합니다. 워크플로
스킬, 전문 역할, MCP/LSP 증거 수집 표면, validator, 릴리스 보조 도구를
묶어 에이전트 작업을 더 잘 지휘하고 더 분명하게 증명할 수 있게 합니다.

Codexy는 한 번의 프롬프트보다 더 많은 구조가 필요한 저장소 작업에
초점을 둡니다.

- 이슈 단위 lane, child worktree, handoff, 리뷰 응답 라우팅, 장시간 검증
  루프를 위한 오케스트레이션 워크플로를 제공합니다.
- 계획, 경로 탐색, QA, sentinel 방식의 준비 상태 점검을 돕는 전문 리뷰
  역할을 제공해 PR 생성이나 handoff 전에 근거를 확인하게 합니다.
- `codegraph`와 `lsp`를 포함한 MCP/LSP 표면을 통해 저장소 증거를 모으고,
  의존 관계를 살피며, 등록된 도구가 현재 세션에서 사용할 수 없을 때 그
  상태를 보고할 수 있게 합니다.
- 플러그인 설정, 마켓플레이스 metadata, 워크플로 계약, touched-file 크기,
  완료 handoff 증거를 점검하는 validator와 릴리스 체크를 제공합니다.
- 브랜치 규율, current-head 리뷰 요청, 상태 체크, 미해결 리뷰 thread,
  merge-ready 증거 패킷을 다루는 증거 기반 GitHub/PR 워크플로를
  지원합니다.

루트 README는 의도적으로 높은 수준의 첫 사용자 안내에 머뭅니다. 실행
가능한 워크플로 규칙은 패키지된 스킬에 두어, 사용자가 운영 설정 세부
절차를 알지 않아도 프로젝트의 역할을 이해할 수 있게 합니다.

## 설치

설정된 Codex 플러그인 마켓플레이스에서 Codexy를 설치한 뒤, 플러그인과 MCP 서버가 보이는지 확인합니다.

```sh
codex plugin add codexy@codexy
codex plugin list
codex mcp list
```

설치한 플러그인 도구가 현재 세션에 보이지 않으면 Codex를 재시작하거나 새 Codex 세션을 여세요.
