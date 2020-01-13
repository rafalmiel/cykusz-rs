fn init() {
    println!("Hello from INIT");
}

fn fini() {
    println!("Bye from FINI");
}

module_init!(init);
module_fini!(fini);


