use syscall_defs::ConnectionFlags;

fn send(fd: usize) -> bool {
    let mut buf = [0u8; 1300];

    //loop {
    //    if let Err(err) = syscall::write(fd, &buf) {
    //        println!("Send failed {:?}", err);

    //        return false
    //    }
    //}

    if let Ok(read) = syscall::read(1, &mut buf) {
        if read == 1 {
            return false;
        }

        //println!("Send {} bytes", read);

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

static mut RECV_BUF: [u8; 2*4096] = [0u8; 2*4096];
static mut SENT: usize = 0;

fn recv(fd: usize) -> bool {
    let res = unsafe { syscall::read(fd, &mut RECV_BUF) };

    match res {
        Ok(len) if len > 1 => {
            //let s = unsafe { core::str::from_utf8_unchecked(&buf[..len]) };

            //print!("{}", s);
            //println!("Sending {} bytes", len);
            //if unsafe {SENT} < 5 * 1024 * 1024 && false {
            //if let Err(e) = unsafe { syscall::write(fd, &RECV_BUF[..len]) } {
            //    println!("Send failed: {:?}", e);
            //}

            unsafe {
                SENT += len;
            }
            //}

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
    unsafe {
        SENT = 0;
    }

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
        } else {
            println!("Select fault, closing");
            break;
        }
    }

    if let Err(e) = syscall::close(fd) {
        println!("Socket close failed: {:?}", e);
    }
}

pub fn connect(port: u32, ip: &[u8]) {
    if let Ok(fd) = syscall::connect(&ip, port, ConnectionFlags::TCP) {
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
