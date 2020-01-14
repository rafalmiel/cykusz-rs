mod i8042;

use spin::Once;

pub trait PS2Controller : Sync {
    fn write_data(&self, byte: u8);
    fn read_status(&self) -> u8;
    fn write_cmd(&self, byte: u8);
}

static CONTROLLER: Once<&'static dyn PS2Controller> = Once::new();

#[allow(unused)]
fn controller() -> &'static dyn PS2Controller{
    let &a = CONTROLLER.r#try().expect("PS2 Controller is not initialised!");
    a
}

pub fn register_controller(ctrl: &'static dyn PS2Controller) {
    CONTROLLER.call_once(|| {
        ctrl
    });
}

fn ps2_init() {
    i8042::init();

    println!("[ OK ] PS/2 Initialised");
}

platform_init!(ps2_init);


