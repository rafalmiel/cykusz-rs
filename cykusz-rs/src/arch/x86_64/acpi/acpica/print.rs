use alloc::string::String;

fn get_uint(Args: &mut core::ffi::VaList, size: usize) -> u64 {
    // (uncheckable) SAFE: Could over-read from stack, returning junk
    unsafe {
        match size {
            0 => Args.arg::<u32>() as u64,
            1 => Args.arg::<u32>() as u64,
            2 => Args.arg::<u64>(),
            _ => unreachable!(),
        }
    }
}
fn get_int(Args: &mut core::ffi::VaList, size: usize) -> i64 {
    // (uncheckable) SAFE: Could over-read from stack, returning junk
    unsafe {
        match size {
            0 => Args.arg::<i32>() as i64,
            1 => Args.arg::<i32>() as i64,
            2 => Args.arg::<i64>(),
            _ => unreachable!(),
        }
    }
}

fn c_string_to_str<'a>(ptr: *const i8) -> &'a str {
    let mut len = 0;

    while unsafe { *ptr.offset(len) } != 0 {
        len += 1;
    }

    let fmt = unsafe { core::slice::from_raw_parts::<u8>(ptr as *const u8, len as usize) };

    let fmt = core::str::from_utf8(fmt).expect("Invalid UTF");

    fmt
}

#[no_mangle]
#[linkage = "external"]
#[allow(dead_code)]
extern "C" fn AcpiOsVprintf(Format: *const i8, mut Args: core::ffi::VaList) {
    let mut out = String::new();

    let fmt = c_string_to_str(Format);

    let mut it = fmt.chars();

    while let Some(mut c) = it.next() {
        if c != '%' {
            out.push(c);
        } else {
            use core::fmt::Write;

            c = match it.next() {
                Some(v) => v,
                _ => return,
            };

            let mut align_left = false;
            if c == '-' {
                align_left = true;
                c = match it.next() {
                    Some(v) => v,
                    _ => return,
                };
            }

            let mut width = 0;
            while let Some(d) = c.to_digit(10) {
                width = width * 10 + d;
                c = match it.next() {
                    Some(v) => v,
                    _ => return,
                };
            }

            let mut precision = !0;
            if c == '.' {
                precision = 0;
                c = match it.next() {
                    Some(v) => v,
                    _ => return,
                };
                while let Some(d) = c.to_digit(10) {
                    precision = precision * 10 + d;
                    c = match it.next() {
                        Some(v) => v,
                        _ => return,
                    };
                }
            }

            let size = if c == 'l' {
                c = match it.next() {
                    Some(v) => v,
                    _ => return,
                };
                if c == 'l' {
                    c = match it.next() {
                        Some(v) => v,
                        _ => return,
                    };
                    2
                } else {
                    1
                }
            } else {
                0
            };

            let _ = align_left;

            match c {
                'x' => {
                    let _ = write!(&mut out, "{:x}", get_uint(&mut Args, size));
                }
                'X' => {
                    let val = get_uint(&mut Args, size);
                    let _ = write!(&mut out, "{:X}", val);
                }
                'd' => {
                    let val = get_int(&mut Args, size);
                    let _ = write!(&mut out, "{}", val);
                }
                'u' => {
                    let val = get_uint(&mut Args, size);
                    let _ = write!(&mut out, "{}", val);
                }
                'p' => {
                    // (uncheckable) SAFE: Could over-read from stack, returning junk
                    let _ = write!(&mut out, "{:p}", unsafe { Args.arg::<*const u8>() });
                }
                'c' => {
                    // (uncheckable) SAFE: Could over-read from stack, returning junk
                    let _ = write!(&mut out, "{}", unsafe { Args.arg::<u32>() as u8 as char });
                }
                's' => {
                    // SAFE: Does as much validation as possible, if ACPICA misbehaves... well, we're in trouble
                    let slice = unsafe {
                        let ptr = Args.arg::<*const u8>();
                        if precision < !0 {
                            ::core::str::from_utf8(::core::slice::from_raw_parts(
                                ptr,
                                precision as usize,
                            ))
                            .unwrap_or("")
                        } else {
                            c_string_to_str(ptr as *const i8)
                        }
                    };
                    let _ = write!(&mut out, "{}", slice);
                }
                _ => {
                    panic!("AcpiOsVprintf - Unknown format code {}", c);
                }
            }
        }
    }

    print!("{}", out);
}
