use std::io::Read;
use std::os::unix::net::UnixListener;

fn main() -> std::io::Result<()> {
    println!("in main");
    let listener = UnixListener::bind("/unix-socket")?;
    println!("got listener");

    loop {
        match listener.accept() {
            Ok((mut s, _addr)) => {
                println!("got client");
                let mut st = String::new();
                s.read_to_string(&mut st)?;
                println!("{}", st);
            },
            Err(e) => {
                println!("accept err: {:?}", e);
            }
        }
    }
}