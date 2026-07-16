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

Codexy는 한 번의 프롬프트만으로는 부족한 저장소 작업을 더 체계적으로 진행하도록
돕는 Codex 하네스 플러그인입니다. 개발자와 팀이 큰 일을 책임과 검토 범위가 분명한 작업
단위로 나누고, 상황에 맞는 작업 담당 또는 검토 에이전트를 활용하며, 안전한
완료에 필요한 검증 기록을 남기도록 돕습니다.

## Codexy가 유용한 때

계획, 구현, 검증, 리뷰, 인수인계까지 이어지는 저장소 작업이나 여러 에이전트의
역할 경계가 분명해야 하는 작업에 Codexy를 사용하세요. Codexy는 이슈 단위 브랜치,
명확한 담당자, 현재 변경이 검토 가능한 상태임을 보여 주는 검증 결과가 필요한
흐름을 위해 만들었습니다.

Codexy에는 다음이 포함됩니다.

- 작업을 분류하고 목표와 계획을 최신 상태로 관리하는 절차 지침
- 구현, 조사, 문서화, 현재 변경 사항 검토를 맡는 전문 역할 정의
- 저장소 탐색과 언어 특성을 반영한 검사를 위한 codegraph와 언어 서버(language
  server) 등록
- 플러그인 설정, pull request 준비 상태, 릴리스 작업을 확인하는 검증기와 GitHub
  기반 검증 절차

## Codexy 설치

Codex 플러그인 마켓플레이스에서 Codexy를 설치합니다. 이 저장소가 아직
마켓플레이스 소스로 등록되어 있지 않다면 먼저 추가하세요.

```sh
codex plugin marketplace add \
  eunsoogi/codexy \
  --ref main
```

그다음 플러그인을 설치합니다.

```sh
codex plugin add codexy@codexy
```

Codex에서 설치된 플러그인과 MCP 서버를 확인합니다.

```sh
codex plugin list
codex mcp list
```

새로 설치한 플러그인, 스킬 또는 MCP가 현재 세션에 나타나지 않으면 Codex를
재시작하거나 새 Codex 세션을 여세요.

## Codexy로 작업하는 흐름

1. **작업을 분류합니다.** 편집 전에 작업 단위, 담당자, 범위, 검증 근거, 중단 조건을
   정합니다.
2. **작업을 의도적으로 진행합니다.** 목표와 계획을 유지하고, 사용할 수 있을 때
   저장소 탐색 및 언어 특성을 반영한 도구를 활용하며, 전문 역할에는 범위가 한정된
   책임을 맡깁니다.
3. **결과를 증명합니다.** 변경한 내용을 검증하고, 현재 커밋을 기준으로 한 검증
   결과를 남기며, pull request가 리뷰와 병합 안전장치를 거치게 합니다.

이 구조에서는 조정 세션이 작업을 배정하고, 하위 작업을 맡은 워크트리 스레드
(worktree thread)가 구현 브랜치와 리뷰 대응 수정을 담당합니다. 범위가 분명한 보조
및 검토 에이전트는 브랜치 담당자가 되지 않고도 작업을 도울 수 있습니다.

## 저장소 유지관리자 안내

Codexy는 플러그인 중심으로 설계되었습니다. 저장소 운영, 패키징, 릴리스,
기여자 규칙은 이 소개 문서에 반복하지 않고 정식 [에이전트 지침](AGENTS.md),
[플러그인 설정 검증기](scripts/validate-plugin-config),
[릴리스 워크플로](.github/workflows/plugin-version-bump.yml)에 둡니다.

## 라이선스

Codexy는 [MIT 라이선스](LICENSE)로 제공됩니다.
