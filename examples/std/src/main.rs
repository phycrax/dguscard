mod uart;

use dguscard_driver::{Resources, Runner};
use embassy_executor::Spawner;
use static_cell::ConstStaticCell;

use crate::uart::UartRx;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let (tx, rx) = uart::new();

    static RESOURCES: ConstStaticCell<Resources> = ConstStaticCell::new(Resources::new(true));
    let resources = RESOURCES.take();

    let runner = dguscard_driver::new(resources, rx, tx);

    spawner.spawn(runner_task(runner).unwrap());

    loop {}
}

#[embassy_executor::task]
async fn runner_task(mut runner: Runner<'static, UartRx>) {
    loop {
        runner.run().await.unwrap();
    }
}
