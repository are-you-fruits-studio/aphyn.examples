//! Many lights on many objects: a floor with 2000 randomly placed lit cubes and spheres, lit by 1024
//! point and spot lights in random colors that drift along slow Lissajous paths.
//!
//! The engine assigns point and spot lights to view clusters every frame, so a fragment only shades
//! the few lights that reach it; that keeps a scene like this interactive. Light ranges are 4 to 10
//! units, and every light fades out smoothly at the edge of its range.
//!
//! Engine modules: `aphyn_render_features` (clustered lighting, lit material template,
//! `StandardLightComponent`, `MeshComponent`, `MaterialComponent`, `CameraComponent`), `aphyn_render`
//! (`create_mesh`, materials, `Camera`), `aphyn_asset_storage` (`AssetStorageResource`), `aphyn_app`
//! (`TimeResource`), `aphyn_transform`, `aphyn_ecs`, `aphyn_math` (`Quat`).

use aphyn::*;

mod mesh;
mod scene;

aphyn_entry_point!(init_app);

pub fn init_app(ctx: AppInitCtx) {
    let AppInitCtx {
        mut data,
        mut behavior,
        window,
        ..
    } = ctx;

    window.size = [1600, 900];
    window.title = "clustered lights".into();

    let mut start = behavior.as_mut();
    let mut start = start.get_mut(Schedule::Start);
    start.insert_system(scene::setup_scene);
    start.order_group(SYSTEM_GROUP_RENDER).before_system(scene::setup_scene);

    let mut update = behavior.as_mut();
    let mut update = update.get_mut(Schedule::Update);
    update.insert_system(scene::animate_lights);
    update
        .order_system(scene::animate_lights)
        .before_group(SYSTEM_GROUP_TRANSFORM);

    scene::spawn_camera(data.as_mut());
}
