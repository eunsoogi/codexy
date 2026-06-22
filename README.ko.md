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

Codexy는 워크플로 중심의 Codex 스킬, `codegraph`와 `lsp` 같은 MCP
래퍼, 전문 리뷰어와 헬퍼 역할 정의, 증거 기반 완료 검증을 위한
validator, 릴리스와 패키징 보조 스크립트를 패키지합니다. 사용 가능한
기능은 설치된 플러그인 버전과 현재 Codex 세션 상태에 따라 달라질 수
있습니다.

## 설치

설정된 Codex 플러그인 마켓플레이스에서 Codexy를 설치한 뒤, 플러그인과 MCP 서버가 보이는지 확인합니다.

```sh
codex plugin add codexy@codexy
codex plugin list
codex mcp list
```

설치한 플러그인 도구가 현재 세션에 보이지 않으면 Codex를 재시작하거나 새 Codex 세션을 여세요.
