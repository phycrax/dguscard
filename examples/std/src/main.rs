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

/// A decoded auto-upload, produced inside the `Events::receive` closure.
///
/// The closure only borrows the queue slot for as long as it runs, so it
/// decodes and hands back an owned value; acting on it (including issuing
/// requests, which needs an `await`) happens after `receive` returns.
enum Input {
    /// Direct speed selection at VP 0x1000.
    Speed(Speed),
    /// Step up/down at VP 0x1001.
    Adjust(Adjust),
    /// Oscillation toggle at VP 0x1002.
    Rotate(bool),
    /// Unmapped frame.
    Ignored,
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

    // Split so the event loop keeps a `Control` to echo state back with.
    let (mut ctl, mut events) = device.split();
    let mut app = FanApp::default();

    loop {
        // Decode inside the closure: it borrows the queue slot, which is
        // released the moment it returns.
        let input = events
            .receive(|mut content| match (content.addr(), content.wlen()) {
                (Address::Word(0x1000), 1) => Input::Speed(content.take().unwrap()),
                (Address::Word(0x1001), 1) => Input::Adjust(content.take().unwrap()),
                // The `bool` decodes from a word (0 = off, 1 = on).
                (Address::Word(0x1002), 1) => Input::Rotate(content.take().unwrap()),
                _ => Input::Ignored,
            })
            .await
            // The runner task panics on a fatal error rather than returning, so
            // a down link here means it is gone for good.
            .expect("runner stopped");

        // Act outside it, where awaiting a request is allowed.
        match input {
            // Direct selection: set the speed to the chosen level.
            Input::Speed(speed) => {
                app.speed = speed;
                println!("speed set to {:?}", app.speed);
            }
            // Step the speed one level up or down, then echo the new level back
            // to the display so its indicator follows.
            Input::Adjust(adjust) => {
                app.speed = match adjust {
                    Adjust::Increase => app.speed.increased(),
                    Adjust::Decrease => app.speed.decreased(),
                };
                println!("speed adjusted to {:?}", app.speed);
                ctl.write(Address::Word(0x1000), &(app.speed as u16))
                    .await
                    .unwrap();
            }
            // Toggle oscillation.
            Input::Rotate(rotating) => {
                app.rotating = rotating;
                println!("rotating: {}", app.rotating);
            }
            Input::Ignored => {}
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
