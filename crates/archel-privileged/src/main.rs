//! Privileged helper stub (per D-03).
//!
//! Plan 02 replaces this stub with the argv-allowlisted helper speaking the
//! versioned JSON protocol. Until then it refuses every invocation with exit
//! code 2 so no caller can mistake the stub for a working elevation path.

fn main() {
    std::process::exit(2);
}
