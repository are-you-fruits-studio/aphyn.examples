//! Custom WGSL shaders: a grid of labelled cells, drawn by shaders written for this demo.
//!
//! Top row: three debug views of the vertex attributes (world normal, uv, world tangent), drawn by
//! one shader through three materials, and a sphere whose left half uses the engine's lit material
//! and whose right half a copy of the engine's lit shader, so the two halves meet without a seam.
//! Bottom row: toon shading with a rim light, computed from the engine's lights buffer and the
//! camera position; four wave tiles displaced in the vertex shader, moving out of phase; and a
//! spinning sphere that dissolves through `discard`, with a glowing edge that blooms and a green
//! core showing through the holes. A warm point light circles the scene.
//!
//! Controls: `F5` loads every shader again from its `.wgsl` file, so a shader edited while the
//! demo runs shows up without a restart (one that fails to compile keeps its previous version, and
//! the error is printed); `B` toggles bloom. The line at the top shows both states.
//!
//! Each of the demo's shaders is an asset: `assets/shaders/<name>.asset` names its `.wgsl` file,
//! which includes the engine's WGSL libraries (`aphyn/standard_types.wgsl`, `aphyn/pbr.wgsl`, ...)
//! and the demo's own ones in `assets/shaders/lib/` by their path under `assets/`. The debug, toon,
//! waves and dissolve materials and their templates are asset files too: a template lists the
//! inputs and maps each one to a member of the shader's uniform at group 1, group 0 being the
//! engine's camera and lights. The copy of the lit shader goes into the engine's standard template
//! from code (`standard_material_template`), so both parity halves take the same material data. The
//! time reaches the animated shaders as a material input that a system writes every frame.
//!
//! Engine modules: `aphyn_render` (shader, material template and material assets, mesh,
//! `StandardVertex`, `Camera`), `aphyn_render_features` (built-in material templates,
//! `standard_material_template`, mesh, material, light and camera components, `BloomResource`,
//! `BatchedMeshComponent`), `aphyn_asset_loading` (asset files, `ShaderLoader`,
//! `AssetsDirResource`), `aphyn_asset_storage`, `aphyn_ui` (`TextComponent`, built-in fonts),
//! `aphyn_transform` (`LocalTransform`, `LocalRectComponent` for the status line), `aphyn_math`
//! (`Quat`), `aphyn_ecs`, `aphyn_app` (`TimeResource`, `InputResource`).

use aphyn::*;

mod mesh;
mod scene;
mod update;

aphyn_entry_point!(init_app);

pub fn init_app(ctx: AppInitCtx) {
    let AppInitCtx {
        mut data,
        mut behavior,
        window,
        ..
    } = ctx;

    window.size = [1280, 720];
    window.title = "Aphyn custom shaders".into();

    data.as_mut().resources_mut().insert(update::DemoState::default());

    let mut start = behavior.as_mut();
    let mut start = start.get_mut(Schedule::Start);
    start.insert_system(scene::setup_scene);
    // the materials and shaders are loaded from asset files, the font comes from the UI module
    start.order_group(SYSTEM_GROUP_ASSETS).before_system(scene::setup_scene);
    start.order_group(SYSTEM_GROUP_UI).before_system(scene::setup_scene);

    let mut update = behavior.as_mut();
    let mut update = update.get_mut(Schedule::Update);
    update.insert_system(update::reload_shaders);
    update.insert_system(update::write_time_into_materials);
    update.insert_system(update::toggle_bloom);
    update.insert_system(update::write_status);
    update.insert_system(update::spin);
    update.insert_system(update::orbit_light);
    update.order_system(update::spin).before_group(SYSTEM_GROUP_TRANSFORM);
    update.order_system(update::orbit_light).before_group(SYSTEM_GROUP_TRANSFORM);
    update.order_system(update::write_status).before_group(SYSTEM_GROUP_UI);

    scene::spawn_camera(data.as_mut());
}
