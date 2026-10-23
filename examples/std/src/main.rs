mod uart;

use dguscard_driver::{Address, Config, Content, Resources, RouteError, Router, RunError, Runner};
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::signal::Signal;
use serde::Deserialize;
use static_cell::{ConstStaticCell, StaticCell};

use crate::uart::UartRx;

/// Largest frame the driver buffers, in bytes.
const MTU: usize = 128;

/// Application state, mutated by the event loop in [`main`].
#[derive(Default, Debug)]
struct FanApp {
    /// Current speed.
    speed: Speed,
    /// Whether the fan head is oscillating.
    rotating: bool,
}

/// Fan speed, auto-uploaded at VP 0x1000 (one word).
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

/// A decoded auto-upload, produced by [`FanRouter`].
enum Event {
    /// Direct speed selection at VP 0x1000.
    Speed(Speed),
    /// Step up/down at VP 0x1001.
    Adjust(Adjust),
    /// Oscillation toggle at VP 0x1002.
    Rotate(bool),
}

/// Decodes the display's auto-uploads and signals them to [`main`].
struct FanRouter {
    events: &'static Signal<NoopRawMutex, Event>,
}

impl Router for FanRouter {
    fn route(&mut self, mut content: Content<'_>) -> Result<(), RouteError> {
        let addr = content.addr();
        let de = |e| RouteError::Deserialize(addr, e);
        let event = match (addr, content.wlen()) {
            (Address::Word(0x1000), 1) => Event::Speed(content.take().map_err(de)?),
            (Address::Word(0x1001), 1) => Event::Adjust(content.take().map_err(de)?),
            // The `bool` decodes from a word (0 = off, 1 = on).
            (Address::Word(0x1002), 1) => Event::Rotate(content.take().map_err(de)?),
            // Fallthrough for unmapped events
            (addr, wlen) => return Err(RouteError::Unhandled(addr, wlen)),
        };

        self.events.signal(event);

        Ok(())
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let (tx, rx) = uart::new();

    static RESOURCES: ConstStaticCell<Resources<NoopRawMutex, MTU>> = ConstStaticCell::new(Resources::new());
    let resources = RESOURCES.take();

    /// Decoded auto-uploads, from the runner task to the event loop in [`main`].
    static EVENTS: StaticCell<Signal<NoopRawMutex, Event>> = StaticCell::new();
    let events: &'static Signal<NoopRawMutex, Event> = EVENTS.init(Signal::new());

    // Display handle owns the tx, runner owns the rx.
    let (mut display, runner) = dguscard_driver::new(resources, rx, tx, Config::new());

    spawner.spawn(runner_task(runner, FanRouter { events }).unwrap());

    // Switch to page 1 (system VP 0x0084), then read the active page back
    // (system VP 0x0014) to confirm it took effect.
    display.write(Address::Word(0x0084), &(0x5A01u16, 1u16)).await.unwrap();
    let page: u16 = display.read(Address::Word(0x0014), 1).await.unwrap();
    println!("current page: {page}");

    let mut app = FanApp::default();

    loop {
        match events.wait().await {
            Event::Speed(speed) => {
                app.speed = speed;
                println!("speed set to {:?}", app.speed);
            }
            Event::Adjust(adjust) => {
                app.speed = match adjust {
                    Adjust::Increase => app.speed.increased(),
                    Adjust::Decrease => app.speed.decreased(),
                };
                println!("speed adjusted to {:?}", app.speed);
                display.write(Address::Word(0x1000), &(app.speed as u16)).await.unwrap();
            }
            Event::Rotate(rotating) => {
                app.rotating = rotating;
                println!("rotating: {}", app.rotating);
            }
        }

        // Print app state
        dbg!(&app);
    }
}

#[embassy_executor::task]
async fn runner_task(mut runner: Runner<'static, NoopRawMutex, UartRx>, mut router: FanRouter) {
    // Buffer for reading into from uart driver.
    let rx_buf = &mut [0; 16];
    // The caller picks the recovery policy for the runner.
    loop {
        // `run` only ever returns `Err`, so this is total.
        let e = runner.run(rx_buf, &mut router).await.unwrap_err();
        match e {
            // Survivable: the offending frame is dropped and the transport is
            // still there, so restart `run`.
            RunError::Accumulation(_) | RunError::Route(_) | RunError::UnmatchedAck | RunError::LateReply(..) => {
                eprintln!("recoverable receive error: {e:?}")
            }
            // The UART itself is gone; there is nothing to recover to.
            RunError::Io(_) | RunError::UnexpectedEof => panic!("receive failed: {e:?}"),
        }
    }
}
