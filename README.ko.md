<p align="center">
  <img src="assets/codexy-agent-hero.png" alt="Codexy" width="100%">
</p>

<h1 align="center">Codexy</h1>

<p align="center">
  Codex 에이전트의 작업을 계획하고 나누어 맡기며, 구현, 검증과 리뷰를 연결하는 하네스
</p>

<p align="center">
  <a href="README.md">English</a>
</p>

<p align="center">
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-2f6f5e.svg"></a>
  <a href="https://github.com/eunsoogi/codexy/commits/main"><img alt="Last commit" src="https://img.shields.io/github/last-commit/eunsoogi/codexy.svg"></a>
  <a href="https://github.com/eunsoogi/codexy/issues"><img alt="GitHub issues" src="https://img.shields.io/github/issues/eunsoogi/codexy.svg"></a>
</p>

Codexy는 Codex 에이전트의 작업을 계획하고 나누어 맡기며, 구현 결과를 실제로
검증하고 리뷰와 인수인계까지 연결하는 Codex 하네스입니다. 자세한 구조와 실행
계약은 연결된 `docs` 문서에서 확인할 수 있습니다.

## getcodexy로 설치하기

`getcodexy`는 컴포넌트 의존성을 계산하고 설치 목록과 수명주기를 관리합니다.

```sh
uv tool install getcodexy
uv tool update-shell
getcodexy install
```

tool bin 디렉터리를 `PATH`에 넣고 셸을 다시 연 뒤 사용하세요. 기본 설치는
`core`, `github`, `devtools`를 모두 포함합니다. 설치나 업데이트 뒤에는 새 Codex
세션을 열어 플러그인, skill, hook, agent, MCP 서버를 host에 노출하세요.

### 컴포넌트 선택

`github`와 `devtools`는 각각 `core`에 의존하며 필요한 의존성은 자동으로
포함됩니다.

| 컴포넌트   | 플러그인          | 추가되는 기능                                                                           |
| ---------- | ----------------- | --------------------------------------------------------------------------------------- |
| `core`     | `codexy`          | 오케스트레이션, 목표와 계획, 담당 worktree, 전문 에이전트, instruction hook, 검증, Wiki |
| `github`   | `codexy-github`   | GitHub workflow context, 좁은 제목 확인, local credential·filesystem·Git safety 확인    |
| `devtools` | `codexy-devtools` | 로컬 Codegraph와 LSP MCP 서버, wrapper, 설정, 개발 도구 지침                            |

| 설치 결과       | 명령                                |
| --------------- | ----------------------------------- |
| core만          | `getcodexy install core`            |
| core + GitHub   | `getcodexy install github`          |
| core + devtools | `getcodexy install devtools`        |
| 전체            | `getcodexy install github devtools` |

### 수명주기 명령

```sh
getcodexy status                       # 설치 목록 확인
getcodexy doctor                       # host 준비 상태와 컴포넌트 상태 확인
uv tool upgrade getcodexy              # CLI 자체 업데이트
getcodexy update                       # 설치된 모든 컴포넌트 업데이트
getcodexy update github                # GitHub 의존 범위 업데이트
getcodexy install github               # GitHub 추가
getcodexy remove github                # 의존 관계가 허용할 때 제거
getcodexy bootstrap                    # 전체 기본 구성으로 수렴
```

모든 명령은 `--json`을 지원합니다. 변경 작업은 journal과 receipt를 남기고,
실패하면 이전 선택을 복원합니다. 선택 규칙과 복구 동작은
[컴포넌트 설치 및 이전 계약](docs/getcodexy-component-installation.md)에
있습니다.

### 기존 monolith 이전

이전은 host가 중개하며, 신뢰할 수 있는 Codex 실행 파일을 절대 경로로 전달해야
합니다.

```sh
getcodexy --codex /absolute/path/to/codex migrate
getcodexy --codex /absolute/path/to/codex migrate core devtools
```

정확히 일치하고 수정되지 않은 versioned legacy tree와 서로 다른 split target만
이전할 수 있습니다. 나머지는 안전하게 거부되며, 중단된 이전은 기존 설정 또는
durable recovery journal을 보존합니다.

### 고급 사용: 플러그인 직접 설치

개발 또는 통제된 복구에서만 사용하고 `core`부터 설치하세요.

```sh
codex plugin marketplace add eunsoogi/codexy --ref v1.7.1
codex plugin add codexy@codexy
codex plugin add codexy-github@codexy
codex plugin add codexy-devtools@codexy
```

등록된 MCP 서버는 `uv`로 공통 부트스트랩을 실행하고, 부트스트랩은 `uvx`로
선택된 release를 실행합니다. Codex를 시작하는 host 환경의 `PATH`에서 `uv`와
`uvx`를 모두 찾을 수 있어야 세션에서 MCP 서버가 실행됩니다. `command -v uv`와
`command -v uvx`(PowerShell에서는 `Get-Command uv`, `Get-Command uvx`)로
확인하세요.

이 명령은 위의 release ref를 대상으로 합니다. 아직 공개되지 않았다면
[Releases](https://github.com/eunsoogi/codexy/releases)에서 공개된 tag를
선택하세요. 아래 기능 설명은 현재 source tree를 기준으로 하며, 변경 사항은
일치하는 공개 release로 설치하세요.

## Codexy가 하는 일

- **담당 범위와 오케스트레이션.** 작업을 분류하고 목표, 계획, issue 단위
  branch/worktree 담당자를 정해 인수인계와 context compaction 뒤에도 근거를
  보존합니다.
- **전문 에이전트와 검증.** 범위에 맞는 specialist를 선택하고, 실행 가능한
  경계에만 TDD를 적용하며 실제 동작과 현재 파일 상태에 근거를 묶습니다.
- **Instruction과 Wiki.** `AGENTS.md` 우선순위를 지키고,
  `init → ingest →
  compile → query → refresh` 흐름으로 출처와 freshness를
  보존하는 Wiki를 운영합니다.
- **GitHub 연동.** GitHub 컴포넌트는 workflow context와 독립적인 local safety
  확인을 제공합니다. 일반 issue·PR·review·merge의 권한과 정책은 host, connector,
  GitHub 및 저장소 소유자가 정합니다. 설치만으로 일반 작업을 차단하거나 PR 본문,
  review 횟수, 고정 승인 문구를 요구하지 않습니다.
- **개발 도구와 복구.** Codegraph와 LSP로 필요한 범위를 탐색하고, 세 컴포넌트의
  설치·업데이트·복구 과정에 receipt와 rollback 근거를 남깁니다.

### 오케스트레이션 한눈에 보기

```mermaid
flowchart TD
    request["요청 또는 issue"] --> classify["범위·담당자·검증 방법 분류"]
    classify --> plan["목표 + 최신 계획"]
    plan --> work["담당 branch/worktree 작업"]
    work --> verify["실제 동작 검증"]
    verify --> review["선택한 리뷰"]
    review --> finish["PR·병합 또는 명시적 인수인계"]
```

### 모델 역할과 추론 수준

Codexy는 작업 담당자와 Codexy에 포함된 전문 에이전트를 구분합니다. 아래는
프로젝트의 역할 설정이며, 설치만으로 호스트의 기본 모델이 바뀌거나 다른 저장소의
GitHub 정책에 동의한 것으로 해석되지 않습니다.

| 역할                       | 모델           | 추론 수준 | 담당 범위                                                                                                                            |
| -------------------------- | -------------- | --------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Orchestrator / parent      | `gpt-6-astra`  | `medium`  | Worker 작업을 배정·추적하고 전체 작업 목표를 맡아 이탈을 교정하며 보고를 검증하고 결과를 인수합니다.                                 |
| Watcher / `codexy-watcher` | `gpt-5.6-luna` | `max`     | 할당된 Worker를 native subagent로 읽기 전용 관찰하고 core Watcher MCP로 중요한 사건을 보고하며, 지시·수정·교체·인수는 하지 않습니다. |
| Worker / ordinary child    | `gpt-5.6-luna` | `max`     | 별도 app task에서 자신의 branch/worktree로 이슈를 구현·검증하고, Watcher 경로가 있으면 지정된 Watcher를 통해 보고합니다.             |

보고 흐름은 Orchestrator가 native Watcher를 호출하고 Worker에게 작업을
배정·교정하면, 해당 경로에서 Worker가 host가 지원하는 task-message 경로로 source
Worker task와 issue/PR 대응을 보존한 채 정확히 지정된 Watcher task에 일반 보고를
보냅니다. Luna/max Watcher는 변경 없는 보고를 억제하고 중요한 사건을
`watcher_report`로 중계하며, Astra/medium Orchestrator는 `watcher_wait`로
받습니다. Orchestrator는 보고를 판단하고 Worker를 지시하며 교정과 결과 인수
권한을 유지합니다. 부모는 Worker에게 구현 지시를 직접 보냅니다. 확인된 메시지
경로 부재나 구체적인 긴급 상황일 때만 한 번 표시된 parent fallback을 허용하며,
일상적인 중복 보고를 되살리지는 않습니다. 목표 전환 receipt는 계속 parent에 직접
보냅니다.

목표도 분리됩니다. Orchestrator는 전체 작업 목표, Watcher는 유한한 관찰 배정,
Worker는 유한한 실행 목표를 맡습니다. Watcher는 전체 목표를 소유하거나 옮기지
않습니다. 이 설정은 Codexy에 포함된 구성이지 이미 실행 중인 host가 실제로 사용한
모델의 증거는 아닙니다. `low`, `medium`, `high`, `xhigh`, `max`는 추론
수준입니다.

### 패키지 전문 에이전트

패키지 catalog는 각 전문 에이전트에 고유한 모델과 추론 수준을 지정하며, 선택형
`codexy-github` 플러그인이 Weaver를 제공합니다.

| 컴포넌트 | 전문 에이전트         | 모델            | 추론 수준 | 담당 범위                                  |
| -------- | --------------------- | --------------- | --------- | ------------------------------------------ |
| core     | `codexy-architect`    | `gpt-6-astra`   | `high`    | 아키텍처와 통합 경계                       |
| core     | `codexy-sentinel`     | `gpt-6-astra`   | `xhigh`   | 엄격한 리뷰                                |
| core     | `codexy-warden`       | `gpt-6-astra`   | `xhigh`   | 안전·권한 경계                             |
| core     | `codexy-inspector`    | `gpt-5.6-sol`   | `medium`  | 표준 리뷰                                  |
| core     | `codexy-auditor`      | `gpt-5.6-terra` | `medium`  | 인수 기준과 실제 동작 검증                 |
| core     | `codexy-cartographer` | `gpt-5.6-luna`  | `low`     | 저장소 탐색                                |
| core     | `codexy-shipwright`   | `gpt-5.6-terra` | `high`    | 릴리스와 패키징                            |
| core     | `codexy-watcher`      | `gpt-5.6-luna`  | `max`     | core Watcher MCP를 통한 native Worker 관찰 |
| github   | `codexy-weaver`       | `gpt-5.6-terra` | `medium`  | GitHub 통합; GitHub 컴포넌트 제공          |

### 실시간 음성 모드

`realtime-voice-orchestration` skill은 일반 `$orchestration`과 함께 사용하는
음성 전용 routing·표현 계층입니다. 담당자, child 조정, 근거, thread 상태의 최종
권한은 일반 오케스트레이션에 있습니다. 확인되지 않은 상태를 추측하지 않고, 원시
log와 불투명한 식별자를 말하지 않으며, PR·merge·release 단계도 구분합니다.

### 상세 문서와 공개 경계

패키지 agent·skill·MCP/LSP runtime의 실제 목록과 LSP batch, hook timing, doctor
상태는 [아키텍처 안내서](docs/architecture.md)에서 확인할 수 있습니다. GitHub의
일반 작업 경계와 선택 가능한 진단은
[GitHub product boundary](docs/plugin-product-boundary.md), 설치 receipt와
오류·복구 규칙은 [설치 계약](docs/getcodexy-component-installation.md)에
있습니다.

## 지원 플랫폼과 검증 범위

| 플랫폼 또는 host surface      | 지원 및 검증 범위                                                                                                                               |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| macOS ARM64 (`darwin-arm64`)  | 세 플러그인의 패키지 대상이며 CI가 build·install과 lifecycle을 검증합니다.                                                                      |
| Linux x86_64 (`linux-x86_64`) | 세 플러그인의 패키지 대상이며 Ubuntu CI가 lifecycle과 migration을 검증합니다.                                                                   |
| Windows x86_64                | CI가 component CLI, transaction lifecycle, recovery, GitHub activation을 실행합니다. 자동 legacy 탐색과 devtools runtime까지 주장하지 않습니다. |
| LSP host prerequisite         | 등록한 language server가 host에 설치되어 실행 가능해야 합니다.                                                                                  |

## 라이선스

Codexy는 [MIT 라이선스](LICENSE)로 제공됩니다.
