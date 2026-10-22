# STM32 NUCLEO-F411RE · Rust · Agentic HIL

VSCode + Rust(Embassy) + probe-rs 기반 펌웨어 개발 파이프라인.
Claude Code가 **코드 수정 → 호스트 테스트 → 크로스 빌드 → 실보드 HIL 테스트 → 로그 분석**을
스스로 반복할 수 있도록 구성되어 있습니다.

```
logic/      no_std 순수 로직 (HAL 의존 없음) — PC에서 cargo test (빠른 루프)
firmware/   Embassy 앱 (src/main.rs) + HIL 테스트 (tests/hil.rs, embedded-test)
scripts/    check.{ps1,sh}: fmt→clippy→host test→build→HIL / run-capture: 플래시+RTT 로그 캡처
.vscode/    rust-analyzer, probe-rs 디버거(launch.json), tasks
.claude/    권한 allowlist, /hil 스킬     CLAUDE.md: 에이전트 작업 규칙
```

## 1. Windows 설치 (최초 1회)

```powershell
# Rust
winget install Rustlang.Rustup
rustup target add thumbv7em-none-eabihf
rustup component add clippy rustfmt

# probe-rs (cargo run / cargo test 러너, 디버거 백엔드)
powershell -ExecutionPolicy Bypass -c "irm https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.ps1 | iex"
probe-rs list        # ST-LINK V2-1 이 보이면 OK
```

- ST-LINK USB 드라이버(STSW-LINK009)가 없으면 설치. ST-LINK 펌웨어가 오래됐으면
  STM32CubeProgrammer 의 ST-LINK upgrade 로 갱신.
- STM32CubeIDE / CubeProgrammer 가 프로브를 잡고 있으면 probe-rs 가 접속 못 합니다.
- VSCode: 폴더 열면 추천 확장(rust-analyzer, probe-rs-debugger, Claude Code) 설치.

## 2. 사용

| 작업 | 명령 |
|---|---|
| 호스트 단위 테스트 | `cd logic; cargo test` |
| 빌드 | `cd firmware; cargo build` |
| 플래시 + defmt 로그 | `cd firmware; cargo run` (Ctrl+C 종료) |
| HIL 테스트 (보드 필요) | `cd firmware; cargo test` |
| 전체 파이프라인 | `powershell -File scripts/check.ps1 -Hil` |
| 디버깅 | VSCode F5 → "Debug firmware (probe-rs)" |

앱 동작: LD2(PA5) 점멸, B1(PC13) 누를 때마다 Slow → Fast → Off, USART2(VCP, 115200)에 모드 출력.

### HIL 테스트 (`firmware/tests/hil.rs`)
| 테스트 | 확인 내용 | 배선 |
|---|---|---|
| `device_id_is_f411` | DBGMCU DEV_ID = 0x431 | 없음 |
| `led_pin_readback` | PA5 출력 → IDR 리드백 | 없음 |
| `button_idle_high` | PC13 풀업 idle High | 없음 (B1 누르지 말 것) |
| `timer_matches_cycle_counter` | embassy-time vs DWT 사이클 카운터 오차 < 2 ms | 없음 |
| `gpio_loopback_pa0_pa1` (`#[ignore]`) | PA0→PA1 루프백 | A0–A1 점퍼, `cargo test -- --ignored` |

probe-rs가 ELF를 한 번 플래시하고 테스트마다 리셋 후 개별 실행, libtest 형식으로 결과를 돌려줍니다.

## 3. Agentic HIL 운용

보드는 **로컬 PC USB**에 붙어 있으므로 HIL 단계는 로컬에서 도는 Claude Code
(VSCode 확장 / CLI / Desktop)로 실행해야 합니다. 클라우드 세션에서는 HIL 이 `SKIP(no-probe)` 됩니다.

1. VSCode 에서 이 폴더를 열고 Claude Code 실행
2. 요청 예: "버튼 롱프레스로 모드 리셋 기능 추가하고 HIL로 검증해줘"
3. 에이전트는 `CLAUDE.md` 루프를 따름: `logic/` 구현+단위테스트 → `firmware/` 연결 →
   `tests/hil.rs` 에 HIL 테스트 추가 → `/hil` (check 스크립트) → `logs/` 분석 → 반복
4. 결과는 `SUMMARY: fmt=PASS ... hil=PASS` 한 줄로 보고

## 4. 다음 단계 아이디어
- 외부 계측 루프백: 두 번째 보드나 USB-UART/로직분석기(sigrok-cli)로 VCP/PWM 파형 자동 검증
- `scripts/` 에 VCP(UART) 캡처 추가 (pyserial) → UART 출력도 HIL assertion 대상으로
- GitHub Actions: host test + build + clippy (HIL 제외), self-hosted runner 에 보드 연결 시 HIL 포함
