mod uart;

use dguscard_driver::{Address, Config, Content, Router, Runner, Stack};
use embassy_executor::Spawner;
use serde::Deserialize;
use static_cell::ConstStaticCell;

use crate::uart::UartRx;

/// Application state; the [`Router`] impl mutates it through `&mut self`.
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

impl Router for FanApp {
    fn route(&mut self, addr: Address, wlen: u8, content: &mut Content<'_>) {
        use Address::*;
        match (addr, wlen) {
            // Direct selection: set the speed to the chosen level.
            (Word(0x1000), 1) => {
                let speed: Speed = content.take().unwrap();
                self.speed = speed;
                println!("speed set to {:?}", self.speed);
            }
            // Step the speed one level up or down, saturating at `Off`/`High`.
            (Word(0x1001), 1) => {
                let adjust: Adjust = content.take().unwrap();
                self.speed = match adjust {
                    Adjust::Increase => self.speed.increased(),
                    Adjust::Decrease => self.speed.decreased(),
                };
                println!("speed adjusted to {:?}", self.speed);
            }
            // Toggle oscillation. The `bool` decodes from a word (0 = off, 1 = on).
            (Word(0x1002), 1) => {
                let rotating: bool = content.take().unwrap();
                self.rotating = rotating;
                println!("rotating: {}", self.rotating);
            }
            _ => {} // ignore unmapped frames
        }
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
    let mut app = FanApp::default();
    loop {
        runner.run(&mut app).await.unwrap();
    }
}
