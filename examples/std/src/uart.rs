use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use serialport::SerialPort;
use static_cell::StaticCell;

type Port = Mutex<NoopRawMutex, Box<dyn SerialPort>>;

pub fn new() -> (UartTx, UartRx) {
    let port = prompt_serial_port().unwrap();
    let port = serialport::new(&port, 115200)
        .open()
        .unwrap_or_else(|e| panic!("failed to open port {port}: {e}"));

    static PORT: StaticCell<Mutex<NoopRawMutex, Box<dyn SerialPort>>> = StaticCell::new();
    let port = PORT.init(Mutex::new(port));

    (UartTx { port }, UartRx { port })
}

fn prompt_serial_port() -> Option<String> {
    let ports = serialport::available_ports().unwrap();

    println!("Choose port:");
    for (idx, port) in ports.iter().enumerate() {
        println!("{}: {:?}", idx, port);
    }

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input");

    let port_idx: Option<usize> = input.trim().parse().ok();

    port_idx.map(|idx| ports[idx].port_name.clone())
}

pub struct UartTx {
    port: &'static Port,
}

pub struct UartRx {
    port: &'static Port,
}

impl embedded_io_async::ErrorType for UartTx {
    type Error = std::io::Error;
}

impl embedded_io_async::Write for UartTx {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.port.lock().await.write_all(buf)?;
        Ok(buf.len())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.port.lock().await.flush()
    }
}

impl embedded_io_async::ErrorType for UartRx {
    type Error = std::io::Error;
}

impl embedded_io_async::Read for UartRx {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        loop {
            let result = self.port.lock().await.read(buf);
            match result {
                Ok(0) => embassy_futures::yield_now().await,
                Ok(n) => break Ok(n),
                Err(_) => {
                    embassy_futures::yield_now().await;
                }
            }
        }
    }
}
