//! Launcher: runs the example in-process, so both `cargo run` and `aphyn build` work.

fn main() {
    aphyn::launch_app_statically(aphynapp::init_app);
}
