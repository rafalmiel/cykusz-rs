use std::process::ExitCode;

fn main() -> Result<(), ExitCode> {
    let mut args = std::env::args();

    if args.len() != 3 {
        println!("Usage: mount <block dev path> <dest dir path>");
        return Err(ExitCode::from(1));
    }

    args.next();

    let source = args.next().unwrap();
    let dest = args.next().unwrap();

    println!("mounting {source} to {dest}");

    syscall_user::mount(
        source.as_str(), dest.as_str(), "ext2"
    ).map_err(|_e| {
        ExitCode::from(1)
    })?;

    return Ok(());
}