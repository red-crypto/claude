---
name: hil
description: Run the full firmware pipeline (fmt, clippy, host tests, build, HIL tests on the attached NUCLEO-F411RE) and diagnose failures. Use after changing firmware or logic code, or when asked to verify on hardware.
---

1. Check the probe: `probe-rs list`. If no ST-LINK is listed, tell the user to plug in the
   board (or close STM32CubeIDE/other debuggers holding the probe), then still run the
   pipeline without HIL and report hardware as NOT verified.
2. Run the pipeline from the repo root:
   - Windows: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1 -Hil`
   - Linux/macOS: `scripts/check.sh --hil`
3. Read the `SUMMARY:` line. For each FAIL, open `logs/check-<ts>/<stage>.log`.
   - `hil`: the log contains libtest-style results plus defmt output per test. Find the
     failing assertion and its `info!` values before touching code.
   - Probe errors (`Probe not found`, `ArmError`, `Timeout`) are environment problems, not
     code bugs: retry once; if the core is locked by a low-power loop, the runner already uses
     `--connect-under-reset`.
4. Fix, re-run only the failing stage first (e.g. `cd firmware && cargo test -- <name>`),
   then the whole pipeline. Report the final SUMMARY line verbatim.
5. To observe the running app instead of tests:
   `scripts/run-capture.sh 10` / `powershell -File scripts/run-capture.ps1 -Seconds 10`.
