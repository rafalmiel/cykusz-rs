
fn init()
{
    println!("Driver init");
}

fn fini()
{
    println!("Driver fini");
}

module_init!(init);
module_fini!(fini);