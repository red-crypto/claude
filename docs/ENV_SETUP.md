# 환경설정 정리 (새 채팅 시작용)

STM32 NUCLEO-F411RE + Rust + VSCode + agentic HIL 펌웨어 파이프라인 환경 요약.
새 채팅을 시작할 때는 맨 아래 **"새 채팅 시작 프롬프트"**를 복사해서 쓰면 됩니다.

---

## 1. 목표
- 보드: NUCLEO-F411RE (STM32F411RET6, Cortex-M4F, Flash 512 KB / RAM 128 KB)
- 언어: Rust (`no_std`, Embassy async)
- IDE: VSCode (rust-analyzer + probe-rs-debugger + Claude Code 확장)
- 파이프라인: 코드 수정 → PC 단위 테스트 → 크로스 빌드 → 실보드 HIL 테스트 → 로그 분석을
  에이전트(Claude Code)가 반복

## 2. 저장소
- GitHub: `red-crypto/claude`, 브랜치 `claude/optimistic-clarke-a61v0j`
- 로컬 경로 예: `C:\Users\user\Documents\STM32_RUST_HIL` (기존 ChatGPT 폴더와 별도로 clone 권장)

```powershell
git clone -b claude/optimistic-clarke-a61v0j https://github.com/red-crypto/claude.git STM32_RUST_HIL
code STM32_RUST_HIL
```

## 3. 툴체인 (Windows, 최초 1회)

| 항목 | 설치 / 확인 |
|---|---|
| Rust (stable, 검증 시 1.97.0) | `winget install Rustlang.Rustup` |
| 타깃 | `rustup target add thumbv7em-none-eabihf` (Cortex-M4F, HW FPU) |
| 컴포넌트 | `rustup component add clippy rustfmt rust-src` |
| probe-rs | `powershell -ExecutionPolicy Bypass -c "irm https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.ps1 \| iex"` |
| ST-LINK 드라이버 | STSW-LINK009 (없을 때만). ST-LINK 펌웨어는 STM32CubeProgrammer로 업그레이드 |
| Claude Code | VSCode 확장 `anthropic.claude-code` 또는 CLI |

확인:
```powershell
rustc --version; cargo --version
rustup target list --installed     # thumbv7em-none-eabihf
probe-rs list                      # ST-LINK V2-1 표시되면 OK
```

> 주의: STM32CubeIDE / CubeProgrammer 가 프로브를 잡고 있으면 probe-rs 접속 실패.

## 4. 크레이트 버전 (검증된 조합)

| crate | version | 비고 |
|---|---|---|
| embassy-stm32 | 0.6.0 | features: `stm32f411re, memory-x, time-driver-any, exti, defmt, unstable-pac` |
| embassy-executor | 0.10.0 | `platform-cortex-m, executor-thread, defmt` (0.10부터 `arch-*` → `platform-*`) |
| embassy-time | 0.5.1 | `defmt, defmt-timestamp-uptime` |
| embassy-sync | 0.8.0 | |
| embassy-futures | 0.1.2 | |
| cortex-m / cortex-m-rt | 0.7.7+ / 0.7.5+ | `critical-section-single-core` |
| defmt / defmt-rtt | 1.x / 1.x | |
| panic-probe | 1.0 | `print-defmt` (앱 전용) |
| embedded-test | 0.7.2 (dev-dep) | `defmt, embassy-010`, 기본 panic-handler 사용 (테스트 전용) |

`memory.x` 는 embassy-stm32 `memory-x` feature 가 자동 생성 → 직접 작성 안 함.

## 5. 프로젝트 구조와 핵심 설정

```
rust-toolchain.toml       stable + thumbv7em-none-eabihf (logic/, firmware/ 에도 동일 파일)
logic/                    no_std, HAL 의존 없음 → 호스트에서 cargo test
firmware/
  .cargo/config.toml      target 고정, runner = probe-rs run --chip STM32F411RETx --connect-under-reset
                          DEFMT_LOG = "debug"
  build.rs                -Tlink.x -Tdefmt.x, 테스트에만 -Tembedded-test.x (rustc-link-arg-tests)
  src/main.rs             Embassy 앱 (LD2 PA5 blink, B1 PC13 EXTI, USART2 PA2 VCP 115200)
  tests/hil.rs            embedded-test HIL 테스트 ([[test]] harness = false)
scripts/
  check.ps1 / check.sh    fmt → clippy(-D warnings) → host test → build → test --no-run → HIL
                          logs/check-<ts>/<stage>.log, 마지막 줄 SUMMARY: ...
  run-capture.ps1 / .sh   플래시 + N초 RTT 로그 캡처, panic/ERROR/HardFault 있으면 FAIL
.vscode/
  settings.json           linkedProjects = firmware, logic / check.allTargets = false / clippy
  launch.json             probe-rs-debug (launch, attach), RTT on, SVD는 선택
  tasks.json              build / run / host test / HIL / pipeline
.claude/
  settings.json           cargo, probe-rs list/info, scripts 허용 / erase·download 는 ask
  skills/hil/SKILL.md     /hil : 파이프라인 실행 + 실패 진단 절차
CLAUDE.md                 에이전트 작업 루프·규칙 (자동 로드)
```

설계 포인트
- **logic 분리**: workspace 로 묶으면 `.cargo/config.toml` 의 target 이 호스트 테스트까지 적용되므로
  `logic/` 과 `firmware/` 를 독립 크레이트로 두고 path 의존성으로 연결.
- **runner 하나로 run/test 겸용**: `probe-rs run` 이 ELF 를 보고 일반 펌웨어/embedded-test 를 자동 판별.
  테스트는 1회 플래시 후 테스트마다 리셋, semihosting 으로 결과 보고 (libtest 형식).
- **panic handler 충돌 회피**: 앱은 `panic_probe`, 테스트 바이너리는 embedded-test 의 panic-handler.
- `--connect-under-reset`: WFI/저전력 루프 중에도 SWD 접속 보장.

## 6. HIL 테스트 현황

| 테스트 | 내용 | 배선 |
|---|---|---|
| device_id_is_f411 | DBGMCU DEV_ID == 0x431 | - |
| led_pin_readback | PA5 출력 → IDR 리드백 | - |
| button_idle_high | PC13 idle High | - (B1 누르지 않기) |
| timer_matches_cycle_counter | embassy-time 100 ms vs DWT, 오차 < 2 ms | - |
| gpio_loopback_pa0_pa1 | `#[ignore]` | A0–A1 점퍼, `cargo test -- --ignored` |

## 7. 검증 상태 (2026-10-22, 클라우드 환경 기준)

| 단계 | 결과 |
|---|---|
| fmt / clippy (-D warnings) | PASS |
| logic host test (2개) | PASS |
| firmware build / HIL 테스트 바이너리 빌드 | PASS (`.embedded_test` 섹션 확인) |
| **실보드 HIL** | **미검증** (클라우드에 보드 없음) |
| **PowerShell 스크립트** | **미실행** (클라우드에 pwsh 없음) |

## 8. 로컬 첫 실행 체크리스트
1. [ ] 3장 툴체인 설치, `probe-rs list` 에 ST-LINK 확인
2. [ ] `cd logic; cargo test` → 2 passed
3. [ ] `cd firmware; cargo build`
4. [ ] `powershell -File scripts/run-capture.ps1 -Seconds 10` → LD2 점멸, `boot: NUCLEO-F411RE` 로그
5. [ ] `powershell -File scripts/check.ps1 -Hil` → `SUMMARY:` 전부 PASS
6. [ ] VSCode F5 "Debug firmware (probe-rs)" 로 브레이크포인트 동작 확인
7. [ ] 터미널(Tera Term 등) COM 포트 115200 → B1 누를 때 `mode: fast` 등 출력

## 9. 트러블슈팅

| 증상 | 원인 / 조치 |
|---|---|
| `Probe not found` / `No probe` | USB 케이블(데이터용), 드라이버, CubeIDE 등 다른 툴 종료 |
| `ArmError` / 접속 타임아웃 | `--connect-under-reset` 적용돼 있음. 보드 RESET 누른 채 재시도 |
| rust-analyzer 가 `can't find crate for test` | `rust-analyzer.check.allTargets = false` 확인, 창 Reload |
| `.ps1` 실행 정책 오류 | `powershell -ExecutionPolicy Bypass -File ...` 로 실행 |
| HIL `button_idle_high` 실패 | 테스트 중 B1 누름 / SB 점퍼 변경 여부 확인 |
| defmt 로그가 깨짐 | 플래시된 ELF 와 로그 디코딩 ELF 불일치 → 재빌드 후 실행 |

## 10. 다음 단계 후보
- UART(VCP) 캡처 스크립트 (pyserial) → UART 출력도 HIL assertion 대상으로
- 로직분석기(sigrok-cli) / 두 번째 보드로 PWM·타이밍 외부 계측
- GitHub Actions: host test + build + clippy, self-hosted runner + 보드 연결 시 HIL 포함
- 기존 ChatGPT 프로젝트의 요구사항·핀맵을 `CLAUDE.md` 의 Target 섹션에 반영

---

## 새 채팅 시작 프롬프트 (복사해서 사용)

**A. 로컬 Claude Code (VSCode 확장/CLI, 보드 연결된 PC) — HIL 가능, 권장**

```
이 저장소는 NUCLEO-F411RE용 Rust(Embassy) 펌웨어 + agentic HIL 파이프라인이야.
CLAUDE.md 와 docs/ENV_SETUP.md 를 먼저 읽고 환경을 파악해줘.
나는 C 위주의 펌웨어 엔지니어라서, Rust 개념은 C와 비교해서 설명해줘.

1) docs/ENV_SETUP.md 8장 "로컬 첫 실행 체크리스트"를 순서대로 실행/확인하고
   (probe-rs list → logic test → build → run-capture → check.ps1 -Hil),
   실패하는 항목은 logs/ 를 읽고 원인과 조치를 정리해줘.
2) 전부 PASS 하면 SUMMARY 줄을 그대로 보여주고, 다음으로 할 작업을 제안해줘.
```

**B. 클라우드(claude.ai/code) 세션 — 빌드/호스트 테스트까지만, HIL 은 SKIP**

```
red-crypto/claude 저장소의 claude/optimistic-clarke-a61v0j 브랜치에서 작업해줘.
NUCLEO-F411RE용 Rust(Embassy) 펌웨어 + agentic HIL 파이프라인이고,
CLAUDE.md 와 docs/ENV_SETUP.md 에 환경이 정리돼 있어. 먼저 읽고 scripts/check.sh 로 현재 상태를 확인해줘.
보드는 내 로컬 PC에 있으니 HIL 이 필요한 변경은 로컬에서 돌릴 수 있게 HIL 테스트까지 작성해두고,
'하드웨어 미검증'이라고 명시해줘. 나는 C 위주 펌웨어 엔지니어야.

이번 작업: <여기에 할 일>
```
