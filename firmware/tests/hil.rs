//! Hardware-in-the-loop tests. Run on the attached NUCLEO-F411RE with:
//!
//!     cargo test            # all HIL tests
//!     cargo test -- <name>  # a single test
//!
//! probe-rs flashes the ELF once, then resets the MCU and runs each test in
//! isolation, reporting libtest-compatible results (and defmt logs) back.
//! Tests marked `#[ignore]` need extra wiring; run them with `-- --ignored`.
#![no_std]
#![no_main]

use defmt_rtt as _;

#[cfg(test)]
#[embedded_test::tests]
mod tests {
    use defmt::{assert, assert_eq, info};
    use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
    use embassy_stm32::Peripherals;
    use embassy_time::{Duration, Instant, Timer};

    #[init]
    fn init() -> Peripherals {
        embassy_stm32::init(Default::default())
    }

    /// MCU identity: DBGMCU_IDCODE.DEV_ID must be 0x431 (STM32F411xC/E).
    #[test]
    fn device_id_is_f411(_p: Peripherals) {
        let idcode = embassy_stm32::pac::DBGMCU.idcode().read();
        info!(
            "DEV_ID=0x{:03x} REV_ID=0x{:04x}",
            idcode.dev_id(),
            idcode.rev_id()
        );
        assert_eq!(idcode.dev_id(), 0x431);
    }

    /// LD2 (PA5): the input data register follows the output we drive.
    #[test]
    fn led_pin_readback(p: Peripherals) {
        let mut led = Output::new(p.PA5, Level::Low, Speed::Low);
        assert!(led.is_set_low());
        led.set_high();
        assert!(
            embassy_stm32::pac::GPIOA.idr().read().idr(5)
                == embassy_stm32::pac::gpio::vals::Idr::HIGH
        );
        led.set_low();
        assert!(
            embassy_stm32::pac::GPIOA.idr().read().idr(5)
                == embassy_stm32::pac::gpio::vals::Idr::LOW
        );
    }

    /// B1 (PC13) has an external pull-up: idle (not pressed) reads high.
    #[test]
    fn button_idle_high(p: Peripherals) {
        let button = Input::new(p.PC13, Pull::None);
        assert!(button.is_high(), "B1 reads low: is it being pressed?");
    }

    /// embassy-time driver ticks at roughly real time (checked against the
    /// DWT cycle counter running on SYSCLK).
    #[test]
    async fn timer_matches_cycle_counter(_p: Peripherals) {
        let mut cp = unsafe { cortex_m::Peripherals::steal() };
        cp.DCB.enable_trace();
        cp.DWT.enable_cycle_counter();

        let sysclk = embassy_stm32::rcc::clocks(&_p.RCC)
            .sys
            .to_hertz()
            .unwrap()
            .0;
        let start_cyc = cortex_m::peripheral::DWT::cycle_count();
        let start = Instant::now();
        Timer::after(Duration::from_millis(100)).await;
        let elapsed_us = start.elapsed().as_micros();
        let cyc = cortex_m::peripheral::DWT::cycle_count().wrapping_sub(start_cyc);
        let cyc_us = (cyc as u64) * 1_000_000 / sysclk as u64;
        info!(
            "sysclk={} Hz timer={} us dwt={} us",
            sysclk, elapsed_us, cyc_us
        );

        assert!(elapsed_us >= 100_000);
        assert!(cyc_us.abs_diff(elapsed_us) < 2_000, "timer drift vs DWT");
    }

    /// Needs a jumper wire between PA0 (A0) and PA1 (A1) on the Arduino header.
    #[test]
    #[ignore]
    async fn gpio_loopback_pa0_pa1(p: Peripherals) {
        let mut out = Output::new(p.PA0, Level::Low, Speed::Low);
        let inp = Input::new(p.PA1, Pull::Down);
        for level in [true, false, true, false] {
            out.set_level(level.into());
            Timer::after_micros(10).await;
            assert_eq!(inp.is_high(), level);
        }
    }
}
