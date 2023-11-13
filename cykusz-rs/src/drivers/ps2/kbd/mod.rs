use crate::arch::int;
use crate::drivers::ps2::PS;
use crate::drivers::ps2::{controller, Command, Error};

pub mod handler;
mod scancode;

#[repr(u8)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
enum KeyboardCommand {
    EnableReporting = 0xF4,
    SetDefaultsDisable = 0xF5,
    SetDefaults = 0xF6,
    Reset = 0xFF,
}

#[repr(u8)]
#[derive(PartialEq, Copy, Clone, Debug)]
#[allow(dead_code)]
enum KeyboardCommandData {
    ScancodeSet = 0xF0,
}

impl PS {
    fn keyboard_command_inner(&self, command: u8) -> Result<u8, Error> {
        self.write(command)?;
        match self.read()? {
            0xFE => Err(Error::CommandRetry),
            value => Ok(value),
        }
    }

    fn keyboard_command(&self, command: KeyboardCommand) -> Result<u8, Error> {
        self.retry(format_args!("keyboard command {:?}", command), 4, |x| {
            x.keyboard_command_inner(command as u8)
        })
    }

    #[allow(dead_code)]
    fn keyboard_command_data(&self, command: KeyboardCommandData, data: u8) -> Result<u8, Error> {
        self.retry(
            format_args!("keyboard command {:?} {:#x}", command, data),
            4,
            |x| {
                let res = x.keyboard_command_inner(command as u8)?;
                if res != 0xFA {
                    //TODO: error?
                    return Ok(res);
                }
                x.write(data)?;
                x.read()
            },
        )
    }
}

fn setup_interrupts() {
    use crate::arch::idt;

    int::set_irq_dest(1, 33);
    idt::add_shared_irq_handler(33, keyboard_interrupt);
}

pub fn init() -> Result<(), Error> {
    handler::init();

    let ps = controller();

    {
        ps.command(Command::EnableFirst)?;
        ps.flush_read();
    }

    {
        let r = ps.keyboard_command(KeyboardCommand::Reset)?;
        if r == 0xFA {
            let b = ps.read().unwrap_or(0);
            if b != 0xAA {
                logln6!("ps2: keyboard faield to self test");
            }
        } else {
            logln6!("ps2: keyboard failed to reset");
        }

        ps.flush_read();
    }

    ps.retry(format_args!("keyboard defaults"), 4, |x| {
        x.flush_read();

        let b = x.keyboard_command(KeyboardCommand::SetDefaultsDisable)?;

        if b != 0xFA {
            return Err(Error::CommandRetry);
        }

        x.flush_read();

        Ok(b)
    })?;

    {
        ps.keyboard_command_inner(KeyboardCommand::EnableReporting as u8)?;
    }

    setup_interrupts();

    Ok(())
}

fn keyboard_interrupt() -> bool {
    handler::handle_interrupt();

    true
}
