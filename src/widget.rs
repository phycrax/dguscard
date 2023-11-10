#[derive(Clone, Copy)]

// callback should be an enum with desired widget callback types, data arg will differ
pub struct Widget {
    addr: u16,
    callback: fn(data: u16),
}

// widget struct, addr and enum type

impl Widget {
    pub fn new(addr: u16, callback: fn(data: u16)) -> Widget {
        Widget { addr, callback }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn callback_test(data: u16) {}

    #[test]
    fn button_callback() {
        let btn = Widget::new(0x5000, callback_test);
    }
}
