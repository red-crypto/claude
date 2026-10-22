//! NUCLEO-F411RE application.
//!
//! - LD2 (PA5): blinks according to the current `BlinkMode`
//! - B1 (PC13, active low): cycles the blink mode
//! - USART2 (PA2/PA3, ST-LINK VCP, 115200 8N1): prints mode changes
//! - defmt over RTT: debug log (shown by `cargo run` / probe-rs)
#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::exti::{self, ExtiInput};
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::usart::{self, UartTx};
use embassy_stm32::{bind_interrupts, interrupt, Config};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use logic::BlinkMode;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    EXTI15_10 => exti::InterruptHandler<interrupt::typelevel::EXTI15_10>;
});

static MODE: Signal<ThreadModeRawMutex, BlinkMode> = Signal::new();

#[embassy_executor::task]
async fn blink(mut led: Output<'static>) {
    let mut mode = BlinkMode::Slow;
    loop {
        match mode.half_period_ms() {
            Some(ms) => {
                led.toggle();
                let t = Timer::after_millis(ms);
                if let embassy_futures::select::Either::Second(m) =
                    embassy_futures::select::select(t, MODE.wait()).await
                {
                    mode = m;
                }
            }
            None => {
                led.set_low();
                mode = MODE.wait().await;
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Config::default());
    info!("boot: NUCLEO-F411RE");

    let led = Output::new(p.PA5, Level::Low, Speed::Low);
    spawner.spawn(blink(led).unwrap());

    let mut tx = UartTx::new_blocking(p.USART2, p.PA2, usart::Config::default()).unwrap();
    let _ = tx.blocking_write(b"boot: NUCLEO-F411RE\r\n");

    let mut button = ExtiInput::new(p.PC13, p.EXTI13, Pull::None, Irqs);
    let mut mode = BlinkMode::Slow;
    loop {
        button.wait_for_falling_edge().await;
        Timer::after_millis(20).await; // debounce
        if button.is_high() {
            continue;
        }
        mode = mode.next();
        info!("mode -> {}", mode);
        MODE.signal(mode);
        let _ = tx.blocking_write(match mode {
            BlinkMode::Slow => b"mode: slow\r\n".as_slice(),
            BlinkMode::Fast => b"mode: fast\r\n".as_slice(),
            BlinkMode::Off => b"mode: off\r\n".as_slice(),
        });
    }
}
