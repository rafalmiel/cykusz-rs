use crate::arch::int;
use crate::drivers::ps2::{controller, Command, Error, PS};

mod handler;

#[repr(u8)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum MouseCommand {
    SetScaling1To1 = 0xE6,
    SetScaling2To1 = 0xE7,
    StatusRequest = 0xE9,
    GetDeviceId = 0xF2,
    EnableReporting = 0xF4,
    SetDefaultsDisable = 0xF5,
    SetDefaults = 0xF6,
    Reset = 0xFF,
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
enum MouseCommandData {
    SetResolution = 0xE8,
    SetSampleRate = 0xF3,
}

impl PS {
    fn mouse_command_inner(&self, command: u8) -> Result<u8, Error> {
        self.command(Command::WriteSecond)?;
        self.write(command)?;
        match self.read()? {
            0xFE => Err(Error::CommandRetry),
            value => Ok(value),
        }
    }

    fn mouse_command(&self, command: MouseCommand) -> Result<u8, Error> {
        self.retry(format_args!("mouse command {:?}", command), 4, |x| {
            x.mouse_command_inner(command as u8)
        })
    }

    #[allow(dead_code)]
    fn mouse_command_data(&self, command: MouseCommandData, data: u8) -> Result<u8, Error> {
        self.retry(
            format_args!("mouse command {:?} {:#x}", command, data),
            4,
            |x| {
                let res = x.mouse_command_inner(command as u8)?;
                if res != 0xFA {
                    //TODO: error?
                    return Ok(res);
                }
                x.command(Command::WriteSecond)?;
                x.write(data as u8)?;
                x.read()
            },
        )
    }
}

fn mouse_interrupt() -> bool {
    handler::handle_interrupt();

    true
}

pub fn init() -> Result<(bool, bool), Error> {
    let ps = controller();

    {
        ps.command(Command::EnableSecond)?;
        ps.flush_read();
    }

    ps.retry(format_args!("mouse reset"), 4, |x| {
        x.flush_read();

        let mut b = x.mouse_command(MouseCommand::Reset)?;
        if b == 0xFA {
            b = x.read()?;
            if b != 0xAA {
                logln6!("mouse self test failed");
                return Err(Error::CommandRetry);
            }
            b = ps.read()?;
            if b != 0x00 {
                logln6!("mouse self test 2 failed");
                return Err(Error::CommandRetry);
            }
        } else {
            logln6!("mouse failed to reset");
            return Err(Error::CommandRetry);
        }

        x.flush_read();
        Ok(b)
    })?;

    {
        let b = ps.mouse_command(MouseCommand::SetDefaults)?;
        if b != 0xFA {
            panic!("mouse failed to set defaults");
        }
        ps.flush_read();
    }

    {
        if ps.mouse_command_data(MouseCommandData::SetSampleRate, 200)? != 0xFA
            || ps.mouse_command_data(MouseCommandData::SetSampleRate, 100)? != 0xFA
            || ps.mouse_command_data(MouseCommandData::SetSampleRate, 80)? != 0xFA
        {
            panic!("mouse failed to enable extra packet");
        }

        ps.flush_read();
    }

    let id = ps.mouse_command(MouseCommand::GetDeviceId)?;
    let mouse_extra = if id == 0xFA {
        ps.read()? == 3
    } else {
        logln6!("failed to get device id");
        false
    };

    ps.flush_read();

    {
        let b = ps.mouse_command_data(MouseCommandData::SetResolution, 3)?;
        if b != 0xFA {
            logln6!("failed to set mouse resolution");
        }
        ps.flush_read();
    }
    {
        let b = ps.mouse_command(MouseCommand::SetScaling1To1)?;
        if b != 0xFA {
            logln6!("failed to set mouse scaling 1to1");
        }
        ps.flush_read();
    }
    {
        let b = ps.mouse_command_data(MouseCommandData::SetSampleRate, 200)?;
        if b != 0xFA {
            logln6!("failed to set mouse sample rate");
        }
        ps.flush_read();
    }
    {
        let b = ps.mouse_command(MouseCommand::StatusRequest)?;
        if b != 0xFA {
            logln6!("failed to request status");
        } else {
            let a = ps.read()?;
            let b = ps.read()?;
            let c = ps.read()?;

            logln6!("mouse status: {} {} {}", a, b, c);
        }
    }

    handler::init();

    ps.mouse_command_inner(MouseCommand::EnableReporting as u8)?;

    use crate::arch::idt;

    int::set_irq_dest(12, 44);
    idt::add_shared_irq_handler(44, mouse_interrupt);

    Ok((true, mouse_extra))
}
