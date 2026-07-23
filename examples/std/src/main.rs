mod uart;

use dguscard_driver::{Config, Runner, Stack, router};
use embassy_executor::Spawner;
use serde::Deserialize;
use static_cell::ConstStaticCell;

use crate::uart::UartRx;

/// Application state, threaded to every handler.
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

/// Direct selection: set the speed to the chosen level.
fn on_speed(fan: &mut FanApp, speed: Speed) {
    fan.speed = speed;
    println!("speed set to {:?}", fan.speed);
}

/// Step the speed one level up or down, saturating at `Off`/`High`.
fn on_adjust(fan: &mut FanApp, adjust: Adjust) {
    fan.speed = match adjust {
        Adjust::Increase => fan.speed.increased(),
        Adjust::Decrease => fan.speed.decreased(),
    };
    println!("speed adjusted to {:?}", fan.speed);
}

/// Toggle oscillation. The `bool` decodes from a word (0 = off, 1 = on).
fn on_rotate(fan: &mut FanApp, rotating: bool) {
    fan.rotating = rotating;
    println!("rotating: {}", fan.rotating);
}

router! {
    name: Router,
    context: FanApp,
    ignore_unhandled: true,
    routes: {
        Word(0x1000), wlen: 1 => on_speed,
        Word(0x1001), wlen: 1 => on_adjust,
        Word(0x1002), wlen: 1 => on_rotate,
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let (tx, rx) = uart::new();

    static RESOURCES: ConstStaticCell<Stack> = ConstStaticCell::new(Stack::new(Config::new()));
    let resources = RESOURCES.take();

    let runner = dguscard_driver::new(resources, rx, tx);

    spawner.spawn(runner_task(runner).unwrap());

    loop {
        embassy_futures::yield_now().await;
    }
}

#[embassy_executor::task]
async fn runner_task(mut runner: Runner<'static, UartRx>) {
    let mut router = Router;
    let mut app = FanApp::default();
    loop {
        runner.run(&mut router, &mut app).await.unwrap();
    }
}
