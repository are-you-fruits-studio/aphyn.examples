//! Launcher: runs the example in-process, so both `cargo run` and `aphyn build` work.

use aphyn::*;

fn main() {
    launch_app_statically(|mut ctx| {
        // `cargo run` makes the engine look in scripts/assets; the example keeps its assets one level up
        if std::env::var_os("APHYN_PROJECT_DIR").is_none()
            && let Some(scripts_dir) = std::env::var_os("CARGO_MANIFEST_DIR")
        {
            let assets_dir = std::path::Path::new(&scripts_dir).join("../assets");
            ctx.data
                .resources_mut()
                .insert(AssetsDirResource(assets_dir.to_string_lossy().as_ref().into()));
        }
        lib_app::init_app(ctx);
    });
}
