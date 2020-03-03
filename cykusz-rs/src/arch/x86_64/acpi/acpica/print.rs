//unsafe fn c_string_to_str<'a>(c_str: *const i8) -> &'a str {
//    ::core::str::from_utf8( ::memory::c_string_as_byte_slice(c_str).unwrap_or(b"INVALID") ).unwrap_or("UTF-8")
//}
//fn get_uint(Args: &mut VaList, size: usize) -> u64 {
//    // (uncheckable) SAFE: Could over-read from stack, returning junk
//    unsafe {
//        match size
//        {
//            0 => Args.arg::<u32>() as u64,
//            1 => Args.arg::<u32>() as u64,
//            2 => Args.arg::<u64>(),
//            _ => unreachable!(),
//        }
//    }
//}
//fn get_int(Args: &mut VaList, size: usize) -> i64 {
//    // (uncheckable) SAFE: Could over-read from stack, returning junk
//    unsafe {
//        match size
//        {
//            0 => Args.arg::<i32>() as i64,
//            1 => Args.arg::<i32>() as i64,
//            2 => Args.arg::<i64>(),
//            _ => unreachable!(),
//        }
//    }
//}

#[no_mangle]
#[linkage = "external"]
#[allow(dead_code)]
extern "C" fn AcpiOsVprintf(Format: *const i8, Args: core::ffi::VaList) {
    /*
    use sync::mutex::LazyMutex;
    struct Buf([u8; 256]);
    impl Buf {
        fn new() -> Self {
            // SAFE: POD
            unsafe { ::core::mem::zeroed() }
        }
    }
    impl AsMut<[u8]> for Buf { fn as_mut(&mut self) -> &mut [u8] { &mut self.0 } }
    impl AsRef<[u8]> for Buf { fn as_ref(&self) -> &[u8] { &self.0 } }
    static TEMP_BUFFER: LazyMutex<::lib::string::FixedString<Buf>> = LazyMutex::new();

    // Acquire input and lock
    // SAFE: Format string is valid for function
    let fmt = unsafe { c_string_to_str(Format) };
    let mut lh = TEMP_BUFFER.lock_init(|| ::lib::string::FixedString::new(Buf::new()));

    // Expand format string
    let mut it = fmt.chars();
    while let Some(mut c) = it.next()
    {
        if c == '\n' {
            // Flush
            println!("{}", *lh);
            lh.clear();
        }
        else if c != '%' {
            lh.push_char(c);
        }
        else {
            use core::fmt::Write;

            c = match it.next() { Some(v)=>v,_=>return };

            let mut align_left = false;
            if c == '-' {
                align_left = true;
                c = match it.next() { Some(v)=>v,_=>return };
            }

            let mut width = 0;
            while let Some(d) = c.to_digit(10) {
                width = width * 10 + d;
                c = match it.next() { Some(v)=>v,_=>return };
            }

            let mut precision = !0;
            if c == '.' {
                precision = 0;
                c = match it.next() { Some(v)=>v,_=>return };
                while let Some(d) = c.to_digit(10) {
                    precision = precision * 10 + d;
                    c = match it.next() { Some(v)=>v,_=>return };
                }
            }

            let size = if c == 'l' {
                c = match it.next() { Some(v)=>v,_=>return };
                if c == 'l' {
                    c = match it.next() { Some(v)=>v,_=>return };
                    2
                }
                else {
                    1
                }
            }
            else {
                0
            };

            // TODO: Use undocumented (but public) APIs in ::core::fmt
            // to create an Arguments structure from this information
            //let spec = ::core::fmt::rt::v1::FormatSpec {
            //	fill: ' ',
            //	align: ::core::fmt::rt::v1::Alignment::Unknown,
            //	flags: 0,
            //	precision: ::core::fmt::rt::v1::Count::Is(precision),
            //	width: ::core::fmt::rt::v1::Count::Is(width),
            //	};
            let _ = align_left;

            match c
            {
                'x' => {
                    let _ = write!(&mut *lh, "{:x}", get_uint(&mut Args, size));
                },
                'X' => {
                    let val = get_uint(&mut Args, size);
                    let _ = write!(&mut *lh, "{:X}", val);
                },
                'd' => {
                    let val = get_int(&mut Args, size);
                    let _ = write!(&mut *lh, "{}", val);
                },
                'u' => {
                    let val = get_uint(&mut Args, size);
                    let _ = write!(&mut *lh, "{}", val);
                },
                'p' => {
                    // (uncheckable) SAFE: Could over-read from stack, returning junk
                    let _ = write!(&mut *lh, "{:p}", unsafe { Args.get::<*const u8>() });
                },
                'c' => {
                    // (uncheckable) SAFE: Could over-read from stack, returning junk
                    let _ = write!(&mut *lh, "{}", unsafe { Args.get::<u32>() as u8 as char });
                },
                's' => {
                    // SAFE: Does as much validation as possible, if ACPICA misbehaves... well, we're in trouble
                    let slice = unsafe {
                        let ptr = Args.get::<*const u8>();
                        if precision < !0 {
                            ::core::str::from_utf8(::core::slice::from_raw_parts(ptr, precision as usize)).unwrap_or("")
                        }
                        else {
                            c_string_to_str(ptr as *const i8)
                        }
                    };
                    let _ = write!(&mut *lh, "{}", slice);
                },
                _ => {
                    panic!("AcpiOsVprintf - Unknown format code {}", c);
                },
            }
        }
    }
    */
}
