use syscall_defs::ConnectionFlags;

fn send(fd: usize) -> bool {
    let mut buf = [0u8; 64];

    if let Ok(read) = syscall::read(1, &mut buf) {
        if read == 1 {
            return false;
        }

        if let Err(err) = syscall::write(fd, &buf[..read]) {
            println!("Send failed {:?}", err);

            false
        } else {
            true
        }
    } else {
        println!("Read failed");

        false
    }
}

fn recv(fd: usize) -> bool {
    let mut buf = [0u8; 64];

    let res = syscall::read(fd, &mut buf);

    match res {
        Ok(len) if len > 1 => {
            let s = unsafe { core::str::from_utf8_unchecked(&buf[..len]) };

            print!("{}", s);

            true
        }
        Err(e) => {
            println!("Read error {:?}", e);

            false
        }
        _ => false,
    }
}

fn start(fd: usize) {
    loop {
        if let Ok(ready) = syscall::select(&[1, fd as u8]) {
            match ready {
                1 => {
                    if !send(fd) {
                        break;
                    }
                }
                _ if ready == fd => {
                    if !recv(fd) {
                        break;
                    }
                }
                _ => {
                    println!("Unexpected fd found");
                }
            }
        }
    }

    if let Err(e) = syscall::close(fd) {
        println!("Socket close failed: {:?}", e);
    }
}

pub fn connect(port: u32, ip: &[u8]) {
    if let Ok(fd) = syscall::connect(&ip, port, ConnectionFlags::UDP) {
        start(fd);
    } else {
        println!("Connect failed");
    }
}

pub fn bind(src_port: u32) {
    if let Ok(fd) = syscall::bind(src_port, ConnectionFlags::TCP) {
        start(fd);
    } else {
        println!("Bind failed");
    }
}
