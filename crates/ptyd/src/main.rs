//! `devpit-ptyd`: the daemon on its own, for running it by hand. The desktop
//! starts the same thing as `devpit ptyd <socket>`; one per user and devpit
//! home.

fn main() {
    std::process::exit(devpit_ptyd::run(std::env::args_os().nth(1)));
}
