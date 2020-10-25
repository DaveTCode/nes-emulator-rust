use log::debug;

#[repr(u8)]
#[derive(Debug)]
pub(crate) enum Controller {
    One,
    Two,
}

#[repr(u8)]
#[derive(Debug, PartialEq)]
pub(crate) enum Button {
    A,
    B,
    Select,
    Start,
    Up,
    Down,
    Left,
    Right,
}

impl Button {
    fn bitflag(&self) -> u8 {
        match self {
            Button::A => 0b0000_0001,
            Button::B => 0b0000_0010,
            Button::Select => 0b0000_0100,
            Button::Start => 0b0000_1000,
            Button::Up => 0b0001_0000,
            Button::Down => 0b0010_0000,
            Button::Left => 0b0100_0000,
            Button::Right => 0b1000_0000,
        }
    }

    fn read_bit(&self, state: u8) -> u8 {
        match self {
            Button::A => self.bitflag() & state,
            Button::B => (self.bitflag() & state) >> 1,
            Button::Select => (self.bitflag() & state) >> 2,
            Button::Start => (self.bitflag() & state) >> 3,
            Button::Up => (self.bitflag() & state) >> 4,
            Button::Down => (self.bitflag() & state) >> 5,
            Button::Left => (self.bitflag() & state) >> 6,
            Button::Right => (self.bitflag() & state) >> 7,
        }
    }

    fn next(&self) -> Option<Self> {
        match self {
            Button::A => Some(Button::B),
            Button::B => Some(Button::Select),
            Button::Select => Some(Button::Start),
            Button::Start => Some(Button::Up),
            Button::Up => Some(Button::Down),
            Button::Down => Some(Button::Left),
            Button::Left => Some(Button::Right),
            Button::Right => None,
        }
    }
}

#[derive(Debug)]
struct ControllerState {
    all_data: u8,
    reading_button: Option<Button>,
}

#[derive(Debug)]
pub(crate) struct Io {
    controller_1_state: ControllerState,
    controller_2_state: ControllerState,
    strobe_register: bool,
}

impl Io {
    pub(crate) fn new() -> Self {
        Io {
            controller_1_state: ControllerState {
                all_data: 0,
                reading_button: Some(Button::A),
            },
            controller_2_state: ControllerState {
                all_data: 0,
                reading_button: Some(Button::A),
            },
            strobe_register: false, // TODO - What is the starting state of the strobe register?
        }
    }

    pub(crate) fn button_down(&mut self, controller: Controller, button: Button) {
        match controller {
            Controller::One => self.controller_1_state.all_data |= button.bitflag(),
            Controller::Two => self.controller_2_state.all_data |= button.bitflag(),
        }
    }

    pub(crate) fn button_up(&mut self, controller: Controller, button: Button) {
        match controller {
            Controller::One => self.controller_1_state.all_data &= !button.bitflag(),
            Controller::Two => self.controller_2_state.all_data &= !button.bitflag(),
        }
    }

    pub(crate) fn read_byte(&mut self, address: u16) -> u8 {
        debug!(
            "Reading from controller register {:04X}, strobing {:}",
            address, self.strobe_register
        );

        fn read_controller_state(state: &mut ControllerState, strobing: bool) -> u8 {
            0x40 | if strobing {
                state.all_data & Button::A.bitflag()
            } else {
                match &state.reading_button {
                    Some(button) => {
                        let result = button.read_bit(state.all_data);
                        state.reading_button = button.next();
                        result
                    }
                    None => 0b0000_0001,
                }
            }
        }

        match address {
            0x4016 => read_controller_state(&mut self.controller_1_state, self.strobe_register),
            0x4017 => read_controller_state(&mut self.controller_2_state, self.strobe_register),
            _ => panic!("Invalid read from io registers {:04X}", address),
        }
    }

    pub(crate) fn write_byte(&mut self, address: u16, value: u8) {
        debug!("Writing to controller register {:04X}={:02X}", address, value);

        match address {
            0x4016 => {
                self.strobe_register = value & 1 == 1;
                self.controller_1_state.reading_button = Some(Button::A);
                self.controller_2_state.reading_button = Some(Button::A);
            }
            _ => panic!("Write to invalid IO register {:04X}={:02X}", address, value),
        }
    }
}
