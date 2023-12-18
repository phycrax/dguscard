struct Device {
    config: &'static Config,
    widgets: Vec<Widget, MAX_WIDGET>,
}

impl Device {
    pub fn new(config: &'static Config) -> Device {
        Device {
            config,
            widgets: Vec::new(),
        }
    }

    pub fn add_widget(&mut self, widget: Widget) {
        self.widgets.push(widget);
    }

    //todo: only accept write cmds, return error otherwise
    pub fn new_packet(&self, cmd: Cmd, addr: u16) -> Packet {
        Packet::new(&self.config, cmd, addr)
    }

    pub fn new_parser(&self) -> Parser {
        Parser::new(&self.config)
    }
}
