mod scancode;
pub mod handler;

use crate::drivers::ps2::PS2Controller;
use crate::drivers::ps2::controller;
use crate::drivers::ps2::PS;
use crate::drivers::ps2::ConfigFlags;

#[repr(u8)]
#[allow(dead_code)]
enum KeyboardCommand {
    Identify = 0xF2,
    EnableReporting = 0xF4,
    SetDefaultsDisable = 0xF5,
    SetDefaults = 0xF6,
    Reset = 0xFF
}

#[repr(u8)]
#[derive(PartialEq, Copy, Clone)]
#[allow(dead_code)]
enum KeyboardCommandData {
    ScancodeSet = 0xF0
}

impl PS {
    fn keyboard_command_inner(&self, command: u8) -> u8 {
        let mut ret = 0xFE;
        for i in 0..4 {
            self.write(command);
            ret = self.read();
            if ret == 0xFE {
                println!("ps2d: retry keyboard command {:X}: {}", command, i);
            } else {
                break;
            }
        }
        ret
    }

    fn keyboard_command(&self, command: KeyboardCommand) -> u8 {
        self.keyboard_command_inner(command as u8)
    }

    #[allow(dead_code)]
    fn keyboard_command_data(&self, command: KeyboardCommandData, data: u8) -> u8 {
        let res = self.keyboard_command_inner(command as u8);
        if res != 0xFA {
            return res;
        }
        self.write(data as u8);
        let res = self.read();
        res
    }
}

fn init() {
    let ctrl = controller();

    use crate::arch::int;

    int::disable();

    if ctrl.keyboard_command(KeyboardCommand::Reset) == 0xFA {
        if ctrl.read() != 0xAA {
            println!("Keyboard self test failed");
        }
    } else {
        println!("Keyboard failed to reset");
    }

    ctrl.flush_read();

    if ctrl.keyboard_command(KeyboardCommand::SetDefaultsDisable) != 0xFA {
        println!("Keyboard scanning disabled failed");
    }

    ctrl.flush_read();

    if ctrl.keyboard_command(KeyboardCommand::EnableReporting) != 0xFA {
        println!("Keyboard failed to enable reporting");
    }

    use crate::arch::idt;

    int::set_irq_dest(1, 33);
    idt::set_handler(33, keyboard_interrupt);

    {
        let mut config = ctrl.config();
        config.remove(ConfigFlags::FIRST_DISABLED);
        config.remove(ConfigFlags::FIRST_TRANSLATE); // Use scancode set 2
        config.insert(ConfigFlags::FIRST_INTERRUPT);
        ctrl.set_config(config);
    }

    ctrl.flush_read();

    println!("[ OK ] Keyboard Initialised");

    int::enable();
}

extern "x86-interrupt" fn keyboard_interrupt(_frame: &mut crate::arch::raw::idt::ExceptionStackFrame) {
    handler::handle_interrupt();

    crate::arch::int::end_of_int();
}

module_init!(init);