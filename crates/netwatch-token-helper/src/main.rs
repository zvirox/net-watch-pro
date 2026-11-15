//! One-shot, fixed-path token reader invoked only through the vendor Polkit action.
//!
//! This binary has no arguments and never accepts a path from its caller.

#![deny(clippy::unwrap_used, clippy::expect_used)]
#![deny(missing_docs)]
#![cfg_attr(not(test), deny(unsafe_code))]

use std::io::Write;

fn main() {
    if std::env::args_os().len() != 1 {
        fail("this helper accepts no arguments");
    }

    match netwatch_token_helper::read_token() {
        Ok(token) => {
            if let Err(error) = writeln!(std::io::stdout().lock(), "{token}") {
                fail(&format!("cannot write token to stdout: {error}"));
            }
        }
        Err(error) => fail(&error),
    }
}

fn fail(message: &str) -> ! {
    let _ = writeln!(std::io::stderr().lock(), "netwatch-token-helper: {message}");
    std::process::exit(1);
}
