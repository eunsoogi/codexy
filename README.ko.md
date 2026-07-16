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

Codexy는 한 번의 프롬프트보다 더 체계적인 흐름이 필요한 저장소 작업을 위한
Codex 하네스 플러그인입니다. 개발자와 팀이 큰 작업을 소유자가 분명하고 리뷰할
수 있는 lane으로 나누고, 알맞은 worker 또는 reviewer 표면을 사용하며, 안전하게
완료하는 데 필요한 증거를 남기도록 돕습니다.

## Codexy가 유용한 때

계획, 구현, 검증, 리뷰, handoff를 함께 거치는 저장소 작업이나 여러 에이전트의
경계가 분명해야 하는 작업에 Codexy를 사용하세요. 이슈 단위 branch, 명확한
소유자, 현재 변경이 준비되었음을 보여 주는 증거가 필요한 흐름을 위해 만들었습니다.

Codexy에는 다음이 포함됩니다.

- 작업을 분류하고 goal과 plan을 최신으로 유지하는 워크플로 지침
- 구현, 조사, 문서화, current-diff review에 집중하는 전문 역할 정의
- 저장소 탐색과 언어 인식 검사를 위한 codegraph 및 language server 등록
- 플러그인 설정, pull request 준비 상태, 릴리스 작업을 확인하는 validator와
  GitHub 중심의 증거 게이트

## Codexy 설치

Codex 플러그인 마켓플레이스를 통해 Codexy를 설치합니다. 이 저장소가 아직
마켓플레이스 소스로 등록되어 있지 않다면 먼저 추가하세요.

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

새로 설치한 플러그인, 스킬, MCP 표면이 현재 세션에 보이지 않으면 Codex를
재시작하거나 새 Codex 세션을 여세요.

## Codexy 워크플로

1. **작업을 분류합니다.** 편집 전에 lane, 소유자, 범위, 증거, 중단 조건을
   정합니다.
2. **의도적으로 lane을 실행합니다.** goal과 plan을 유지하고, 가능할 때 저장소
   및 언어 인식 도구를 사용하며, 전문 역할에 제한된 책임을 맡깁니다.
3. **결과를 증명합니다.** 바뀐 표면을 검증하고 current-head 증거를 남기며, pull
   request가 리뷰와 merge 안전장치를 거치게 합니다.

이 구조에서는 조정 세션이 작업을 라우팅하고, child worktree thread가 구현
branch와 리뷰 대응 수정을 소유합니다. 집중된 helper와 reviewer agent는 branch
소유자가 되지 않고도 작업을 도울 수 있습니다.

## 저장소 유지관리자 안내

Codexy는 플러그인 우선 구조입니다. 이 소개 문서에는 저장소 거버넌스, 패키징,
릴리스, 기여자 규칙을 반복하지 않고, 정식 [에이전트 지침](AGENTS.md),
[플러그인 설정 검증기](scripts/validate-plugin-config),
[릴리스 워크플로](.github/workflows/plugin-version-bump.yml)에 둡니다.

## 라이선스

Codexy는 [MIT 라이선스](LICENSE)로 제공됩니다.
