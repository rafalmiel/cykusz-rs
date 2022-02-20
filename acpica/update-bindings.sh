#!/usr/bin/env bash

bindgen  --no-derive-debug --use-core -o src/raw64.rs --whitelist-type acpi.* --whitelist-var ACPI.*  --whitelist-function Acpi.* --blacklist-function AcpiOs.*  --no-layout-tests acpica/source/include/acpi.h  -- -DCYKUSZ -DACPI_LIBRARY

function replacements() {
    sed $1 -i -e 's/::std::os::raw::c_char/i8/'
    sed $1 -i -e 's/::std::os::raw::c_uchar/i8/'
    sed $1 -i -e 's/::std::os::raw::c_void/c_void/'
    sed $1 -i -e 's/::std::os::raw::c_int/i32/'
    sed $1 -i -e 's/::std::os::raw::c_long/i32/'
    sed $1 -i -e 's/::std::os::raw::c_uint/u32/'
    sed $1 -i -e 's/::std::os::raw::c_ulong/u32/'
    sed $1 -i -e 's/BOOLEAN = .*;/BOOLEAN = bool;/'
    sed $1 -i -e 's/UINT8 = .*;/UINT8 = u8;/'
    sed $1 -i -e 's/UINT16 = .*;/UINT16 = u16;/'
    sed $1 -i -e 's/UINT32 = .*;/UINT32 = u32;/'
    sed $1 -i -e 's/UINT64 = .*;/UINT64 = u64;/'
    sed $1 -i -e 's/INT16 = .*;/INT16 = i16;/'
    sed $1 -i -e 's/INT32 = .*;/INT32 = i32;/'
    sed $1 -i -e 's/INT64 = .*;/INT64 = i64;/'
    sed $1 -i -e 's/::std::option/::core::option/'
    sed $1 -i -e 's/::std::mem/::core::mem/'
    sed $1 -i -e 's/::std::clone/::core::clone/'
    sed $1 -i -e 's/::std::default/::core::default/'
    sed $1 -i -e 's/ = 1, }/ = 1, _UNUSED_VARIANT = 2, }/'
}

replacements src/raw64.rs
