macro_rules! init_arr (
    ($a: ident, $([$k: expr, $v: expr]),+) => {
        $($a[$k] = $v;)*
    };
);

pub static MAP: [char; 127] = {
    let mut arr = ['\0'; 127];

    init_arr!(arr,
        [0x1c, 'a'],
        [0x32, 'b'],
        [0x21, 'c'],
        [0x23, 'd'],
        [0x24, 'e'],
        [0x2b, 'f'],
        [0x34, 'g'],
        [0x33, 'h'],
        [0x43, 'i'],
        [0x3b, 'j'],
        [0x42, 'k'],
        [0x4b, 'l'],
        [0x3a, 'm'],
        [0x31, 'n'],
        [0x44, 'o'],
        [0x4d, 'p'],
        [0x15, 'q'],
        [0x2d, 'r'],
        [0x1b, 's'],
        [0x2c, 't'],
        [0x3c, 'u'],
        [0x2a, 'v'],
        [0x1d, 'w'],
        [0x22, 'x'],
        [0x35, 'y'],
        [0x1a, 'z'],
        [0x45, '0'],
        [0x16, '1'],
        [0x1e, '2'],
        [0x26, '3'],
        [0x25, '4'],
        [0x2e, '5'],
        [0x36, '6'],
        [0x3d, '7'],
        [0x3e, '8'],
        [0x46, '9'],
        [0x0e, '`'],
        [0x4e, '-'],
        [0x55, '='],
        [0x5d, '\\'],
        [0x29, ' '],
        [0x0d, '\t'],
        [0x5a, '\n'],
        [0x54, '['],
        [0x5b, ']'],
        [0x4c, ';'],
        [0x52, '\''],
        [0x41, ','],
        [0x49, '.'],
        [0x4a, '/']
    );

    arr
};

pub fn get(sc: usize) -> char{
    MAP[sc]
}

