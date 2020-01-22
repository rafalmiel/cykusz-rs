mod i8042;
pub mod kbd;

use spin::Once;
use crate::drivers::ps2::Command::{DisableFirst, DisableSecond};

pub trait PS2Controller : Sync {
    fn write(&self, byte: u8);
    fn read(&self) -> u8;
    fn command(&self, byte: Command);
    fn status(&self) -> StatusFlags;
}

struct PS {
    ops: &'static dyn PS2Controller
}

static CONTROLLER: Once<PS> = Once::new();

fn controller() -> &'static PS {
    CONTROLLER.r#try().expect("PS2 Controller is not initialised!")
}

pub fn register_controller(ctrl: &'static dyn PS2Controller) {
    CONTROLLER.call_once(|| {
        PS { ops: ctrl }
    });
}

bitflags! {
    pub struct StatusFlags: u8 {
        const OUTPUT_FULL = 1;
        const INPUT_FULL = 1 << 1;
        const SYSTEM = 1 << 2;
        const COMMAND = 1 << 3;
        const KEYBOARD_LOCK = 1 << 4;
        const SECOND_OUTPUT_FULL = 1 << 5;
        const TIME_OUT = 1 << 6;
        const PARITY = 1 << 7;
    }
}

bitflags! {
    pub struct ConfigFlags: u8 {
        const FIRST_INTERRUPT = 1;
        const SECOND_INTERRUPT = 1 << 1;
        const POST_PASSED = 1 << 2;
        // 1 << 3 should be zero
        const CONFIG_RESERVED_3 = 1 << 3;
        const FIRST_DISABLED = 1 << 4;
        const SECOND_DISABLED = 1 << 5;
        const FIRST_TRANSLATE = 1 << 6;
        // 1 << 7 should be zero
        const CONFIG_RESERVED_7 = 1 << 7;
    }
}

#[repr(u8)]
#[allow(dead_code)]
pub enum Command {
    ReadConfig = 0x20,
    WriteConfig = 0x60,
    DisableSecond = 0xA7,
    EnableSecond = 0xA8,
    TestSecond = 0xA9,
    TestController = 0xAA,
    TestFirst = 0xAB,
    Diagnostic = 0xAC,
    DisableFirst = 0xAD,
    EnableFirst = 0xAE,
    WriteSecond = 0xD4
}

impl PS {

    fn wait_write(&self) {
        while self.status().contains(StatusFlags::INPUT_FULL) {}
    }

    fn wait_read(&self) {
        while ! self.status().contains(StatusFlags::OUTPUT_FULL) {}
    }

    fn flush_read(&self) {
        while self.status().contains(StatusFlags::OUTPUT_FULL) {
            let _ = self.read();
        }
    }

    fn config(&self) -> ConfigFlags {
        self.command(Command::ReadConfig);
        ConfigFlags::from_bits_truncate(self.read())
    }

    fn set_config(&self, config: ConfigFlags) {
        self.command(Command::WriteConfig);
        self.write(config.bits());
    }
}

impl PS2Controller for PS {

    fn write(&self, byte: u8) {
        self.wait_write();
        self.ops.write(byte);
    }

    fn read(&self) -> u8 {
        self.wait_read();
        self.ops.read()
    }

    fn command(&self, byte: Command) {
        self.wait_write();
        self.ops.command(byte)
    }

    fn status(&self) -> StatusFlags {
        self.ops.status()
    }
}

fn init() {
    let ctrl = controller();

    ctrl.flush_read();

    ctrl.command(DisableFirst);
    ctrl.command(DisableSecond);

    ctrl.flush_read();

    {
        let mut config = ctrl.config();
        config.insert(ConfigFlags::FIRST_DISABLED);
        config.insert(ConfigFlags::SECOND_DISABLED);
        config.remove(ConfigFlags::FIRST_INTERRUPT);
        config.remove(ConfigFlags::SECOND_INTERRUPT);
        ctrl.set_config(config);
    }

    ctrl.command(Command::TestController);
    let read = ctrl.read();
    if read != 0x55 {
        panic!("[ ERROR ] Could not initialise PS/2 - Self Test Failed (got: 0x{:x})", read);
    }

    ctrl.command(Command::EnableFirst);
    //ctrl.command(Command::EnableSecond);

    ctrl.flush_read();
}

fn ps2_init() {
    i8042::init();

    init();

    println!("[ OK ] PS/2 Initialised");
}

platform_init!(ps2_init);


