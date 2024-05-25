use std::process::ExitCode;

fn main() -> Result<(), ExitCode> {
    let mut args = std::env::args();

    if args.len() != 2 {
        println!("Usage: umount <dest dir path>");
        return Err(ExitCode::from(1));
    }

    args.next();

    let dest = args.next().unwrap();

    println!("unmounting {dest}");

    syscall_user::umount(
        dest.as_str()
    ).map_err(|_e| {
        ExitCode::from(1)
    })?;

    return Ok(());
}