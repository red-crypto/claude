# STM32 NUCLEO-F411RE · Rust firmware · agentic HIL

## Target
- Board: NUCLEO-F411RE (STM32F411RET6, Cortex-M4F, 512 KB flash / 128 KB RAM, 16 MHz HSI default)
- Probe: on-board ST-LINK/V2-1 (SWD) via probe-rs, chip name `STM32F411RETx`
- Board I/O: LD2 = PA5 (active high), B1 = PC13 (active low, external pull-up),
  USART2 PA2/PA3 = ST-LINK virtual COM port

## Layout
- `logic/`    — `no_std`, HAL-free logic. Runs on the **host** (`cargo test`). Put state machines,
                parsers, filters, protocol code here first.
- `firmware/` — Embassy app (`src/main.rs`) + HIL tests (`tests/hil.rs`, embedded-test).
                Target is fixed by `firmware/.cargo/config.toml` (thumbv7em-none-eabihf),
                runner is `probe-rs run`.
- `scripts/`  — pipeline scripts (`check.*`, `run-capture.*`), `.ps1` for Windows, `.sh` otherwise.
- `logs/`     — script output (git-ignored). Read these instead of re-running when possible.

## Commands (run from repo root)
| Purpose | Windows | Linux/macOS |
|---|---|---|
| Full pipeline | `powershell -File scripts/check.ps1 [-Hil]` | `scripts/check.sh [--hil]` |
| Host unit tests | `cd logic; cargo test` | same |
| Firmware build | `cd firmware; cargo build` | same |
| HIL tests (board) | `cd firmware; cargo test` | same |
| One HIL test | `cd firmware; cargo test -- <name>` | same |
| Flash + capture log | `powershell -File scripts/run-capture.ps1 -Seconds 10` | `scripts/run-capture.sh 10` |
| Probe present? | `probe-rs list` | same |

Never run bare `cargo run` from an agent — it streams RTT forever. Use `run-capture`.

## Development loop (agent)
1. Put/modify logic in `logic/` with host unit tests → `cargo test` there (seconds).
2. Wire it in `firmware/` → `cargo build` + `cargo clippy -- -D warnings`.
3. Add/adjust a HIL test in `firmware/tests/hil.rs` that checks the real hardware behaviour
   (register readback, timing via DWT, pin loopback, ...).
4. Run the full pipeline with HIL. Only call a task done when `SUMMARY:` shows all PASS.
   If `hil=SKIP(no-probe)`, say explicitly that hardware was **not** verified.
5. On failure, read `logs/check-*/<stage>.log` and the defmt output before changing code.

## Rules
- Keep `logic/` free of HAL/embassy deps so it stays host-testable.
- Don't change clocks (`embassy_stm32::Config.rcc`), linker/memory settings, or probe-rs
  runner flags without saying why; they affect every test.
- Tests that need extra wiring are `#[ignore]` and documented with the required jumper.
- Don't flash anything that disables SWD (e.g. remapping PA13/PA14) or enables read-out
  protection / option bytes changes.
- HIL tests run one by one with a reset in between; each gets fresh `Peripherals` from `#[init]`.
- Use `defmt::info!/debug!` for logs; `DEFMT_LOG` is set in `firmware/.cargo/config.toml`.
