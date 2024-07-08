use core::hint::spin_loop;

use spin::Once;

use crate::kernel::sync::IrqGuard;

mod i8042;
pub mod kbd;
pub mod mouse;

fn pause() {
    spin_loop();
}

#[derive(Debug)]
pub enum Error {
    CommandRetry,
    NoMoreTries,
    ReadTimeout,
    WriteTimeout,
}

pub trait PS2Controller: Sync {
    fn write(&self, byte: u8);
    fn read(&self) -> u8;
    fn command(&self, byte: Command);
    fn status(&self) -> StatusFlags;
}

struct PS {
    ops: &'static dyn PS2Controller,
}

static CONTROLLER: Once<PS> = Once::new();

fn controller() -> &'static PS {
    CONTROLLER
        .get()
        .expect("PS2 Controller is not initialised!")
}

pub fn register_controller(ctrl: &'static dyn PS2Controller) {
    CONTROLLER.call_once(|| PS { ops: ctrl });
}

bitflags! {
    #[derive(Copy, Clone)]
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
    #[derive(Copy, Clone)]
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
    WriteSecond = 0xD4,
}

impl PS {
    fn wait_write(&self) -> Result<(), Error> {
        let mut timeout = 1_000_000;
        while timeout > 0 {
            if !self.status().contains(StatusFlags::INPUT_FULL) {
                return Ok(());
            }
            pause();
            timeout -= 1;
        }
        Err(Error::WriteTimeout)
    }

    fn wait_read(&self) -> Result<(), Error> {
        let mut timeout = 1_000_000;
        while timeout > 0 {
            if self.status().contains(StatusFlags::OUTPUT_FULL) {
                return Ok(());
            }
            pause();
            timeout -= 1;
        }
        Err(Error::ReadTimeout)
    }

    fn flush_read(&self) {
        let mut timeout = 100;
        while timeout > 0 && self.status().contains(StatusFlags::OUTPUT_FULL) {
            let _ = self.ops.read();
            pause();
            timeout -= 1;
        }
    }

    #[allow(dead_code)]
    fn config(&self) -> Result<ConfigFlags, Error> {
        self.retry(format_args!("read config"), 4, |x| {
            x.command(Command::ReadConfig)?;
            x.read()
        })
        .map(ConfigFlags::from_bits_truncate)
    }

    fn set_config(&self, config: ConfigFlags) -> Result<(), Error> {
        self.retry(format_args!("read config"), 4, |x| {
            x.command(Command::WriteConfig)?;
            x.write(config.bits())?;
            Ok(0)
        })?;
        Ok(())
    }

    fn write(&self, byte: u8) -> Result<(), Error> {
        self.wait_write()?;
        self.ops.write(byte);
        Ok(())
    }

    fn read(&self) -> Result<u8, Error> {
        self.wait_read()?;
        Ok(self.ops.read())
    }

    fn command(&self, byte: Command) -> Result<(), Error> {
        self.wait_write()?;
        self.ops.command(byte);
        Ok(())
    }

    fn status(&self) -> StatusFlags {
        self.ops.status()
    }

    fn retry<F: Fn(&Self) -> Result<u8, Error>>(
        &self,
        name: core::fmt::Arguments,
        retries: usize,
        f: F,
    ) -> Result<u8, Error> {
        let mut res = Err(Error::NoMoreTries);
        for retry in 0..retries {
            res = f(self);
            match res {
                Ok(ok) => {
                    return Ok(ok);
                }
                Err(ref err) => {
                    logln6!("ps2d: {}: retry {}/{}: {:?}", name, retry + 1, retries, err);
                }
            }
        }
        res
    }
}

fn init() -> Result<(), Error> {
    let _irq = IrqGuard::new();

    let ps = controller();

    ps.flush_read();

    {
        ps.command(Command::DisableFirst)?;
        ps.command(Command::DisableSecond)?;

        ps.flush_read();
    }

    let mut config;
    {
        config =
            ConfigFlags::POST_PASSED | ConfigFlags::FIRST_DISABLED | ConfigFlags::SECOND_DISABLED;
        ps.set_config(config)?;

        ps.flush_read();
    }

    let keyboard_found = kbd::init().is_ok();

    if !keyboard_found {
        panic!("no keyboard!");
    }

    let (mouse_found, _mouse_extra) = match mouse::init() {
        Err(_) | Ok((false, _)) => (false, false),
        Ok((true, extra)) => (true, extra),
    };

    {
        if keyboard_found {
            config.remove(ConfigFlags::FIRST_DISABLED);
            config.insert(ConfigFlags::FIRST_INTERRUPT);
        } else {
            config.insert(ConfigFlags::FIRST_DISABLED);
            config.remove(ConfigFlags::FIRST_INTERRUPT);
        }
        if mouse_found {
            config.remove(ConfigFlags::SECOND_DISABLED);
            config.insert(ConfigFlags::SECOND_INTERRUPT);
        } else {
            config.insert(ConfigFlags::SECOND_DISABLED);
            config.remove(ConfigFlags::SECOND_INTERRUPT);
        }
        if let Err(e) = ps.set_config(config) {
            logln6!("ps2: Failed to set config: {:?}", e);
        }
    }

    ps.flush_read();

    Ok(())
}

fn ps2_init() {
    i8042::init();

    if let Ok(()) = init() {
        println!("[ OK ] PS/2 Initialised");
    }
}

platform_init!(ps2_init);
