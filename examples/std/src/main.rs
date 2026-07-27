mod uart;

use dguscard_driver::{Address, Config, Error, Resources, Runner};
use embassy_executor::Spawner;
use serde::Deserialize;
use static_cell::ConstStaticCell;

use crate::uart::UartRx;

/// Application state, mutated by the event loop in [`main`].
#[derive(Default)]
struct FanApp {
    /// Current speed.
    speed: Speed,
    /// Whether the fan head is oscillating.
    rotating: bool,
}

/// Fan speed, auto-uploaded at VP 0x1000 (one word).
///
/// Unit variants decode from their index, so `Off = 0 .. High = 3`.
#[derive(Deserialize, Debug, Default, Clone, Copy)]
enum Speed {
    #[default]
    Off,
    Low,
    Medium,
    High,
}

impl Speed {
    /// Next higher speed, saturating at `High`.
    fn increased(self) -> Self {
        match self {
            Speed::Off => Speed::Low,
            Speed::Low => Speed::Medium,
            Speed::Medium | Speed::High => Speed::High,
        }
    }

    /// Next lower speed, saturating at `Off`.
    fn decreased(self) -> Self {
        match self {
            Speed::High => Speed::Medium,
            Speed::Medium => Speed::Low,
            Speed::Low | Speed::Off => Speed::Off,
        }
    }
}

/// Increase/decrease event at VP 0x1001 (one word): `Decrease = 0`, `Increase = 1`.
#[derive(Deserialize)]
enum Adjust {
    Decrease,
    Increase,
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let (tx, rx) = uart::new();

    static RESOURCES: ConstStaticCell<Resources> = ConstStaticCell::new(Resources::new());
    let resources = RESOURCES.take();

    // The device owns the UART transmitter and writes to it inline; the runner
    // owns the receiver and feeds completions and events back over the channels.
    let (mut device, runner) = dguscard_driver::new(resources, rx, tx, Config::new());

    spawner.spawn(runner_task(runner).unwrap());

    // Switch to page 1 (system VP 0x0084), then read the active page back
    // (system VP 0x0014) to confirm it took effect.
    device
        .write(Address::Word(0x0084), &(0x5A01u16, 1u16))
        .await
        .unwrap();
    let page: u16 = device.read(Address::Word(0x0014), 1).await.unwrap();
    println!("current page: {page}");

    // Split so the event loop can issue writes while an `Event` guard is alive.
    let (mut ctl, mut events) = device.split();
    let mut app = FanApp::default();

    loop {
        let mut event = events.receive().await;
        match (event.addr(), event.wlen()) {
            // Direct selection: set the speed to the chosen level.
            (Address::Word(0x1000), 1) => {
                app.speed = event.take().unwrap();
                println!("speed set to {:?}", app.speed);
            }
            // Step the speed one level up or down, then echo the new level back
            // to the display so its indicator follows.
            (Address::Word(0x1001), 1) => {
                app.speed = match event.take().unwrap() {
                    Adjust::Increase => app.speed.increased(),
                    Adjust::Decrease => app.speed.decreased(),
                };
                println!("speed adjusted to {:?}", app.speed);
                ctl.write(Address::Word(0x1000), &(app.speed as u16))
                    .await
                    .unwrap();
            }
            // Toggle oscillation. The `bool` decodes from a word (0 = off, 1 = on).
            (Address::Word(0x1002), 1) => {
                app.rotating = event.take().unwrap();
                println!("rotating: {}", app.rotating);
            }
            _ => {} // ignore unmapped frames
        }
    }
}

#[embassy_executor::task]
async fn runner_task(mut runner: Runner<'static, UartRx>) {
    let rx_buf = &mut [0; 16];
    // The caller picks the recovery policy: framing noise and event-queue
    // overflow are survivable by restarting `run`; a read error means the UART
    // itself is gone. Write errors surface at the `Control` call site instead.
    loop {
        match runner.run(rx_buf).await {
            Err(Error::Accumulation(e)) => eprintln!("framing error, resyncing: {e:?}"),
            Err(Error::EventQueueFull) => eprintln!("event queue full, frame dropped"),
            Err(e) => panic!("receive failed: {e:?}"),
            Ok(_infallible) => unreachable!(),
        }
    }
}
