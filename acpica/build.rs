/* Copyright (c) 2015 The Robigalia Project Developers
 * Licensed under the Apache License, Version 2.0
 * <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT
 * license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
 * at your option. All files in the project carrying such
 * notice may not be copied, modified, or distributed except
 * according to those terms.
 */

use std::env;
use std::process::Command;
fn main() {
    assert!(Command::new("make")
        .arg("-f")
        .arg("_Makefile")
        .status()
        .unwrap()
        .success());

    println!(
        "cargo:rustc-link-lib=static=acpica-{}",
        env::var("TARGET").unwrap()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        env::var("OUT_DIR").unwrap()
    );
}
