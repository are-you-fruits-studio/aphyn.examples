//! A voxel house with a tree next to it, 483 voxels on a ground slab under two directional lights,
//! where every voxel is its own cube entity. Space or Enter blows the house apart: each cube gets an
//! outward and upward kick and a spin, falls under gravity, bounces on the ground and settles flat.
//! Tab or R snaps every cube back to its place. Keys 1-9 rebuild the house at cube level N, with
//! every voxel split into N^3 smaller cubes: 3 864 cubes at the starting level 2, 13 041 at level 3,
//! 60 375 at level 5 and 352 107 at level 9. The camera orbits the house; a HUD line at the bottom
//! lists the keys and one at the top counts the cubes.
//!
//! A stress test for many entities: every cube has its own transform, mesh and material
//! components, and a rebuild destroys every cube entity and spawns the new ones at runtime. High
//! levels are slow to rebuild and to render. There is no collision between cubes, so piles of
//! debris interpenetrate. The explosion uses simple hand-written physics on purpose: `aphyn_physics`
//! rigidbodies would turn this entity and rendering stress test into a benchmark of the physics
//! solver, which is much slower at these counts.
//!
//! Engine modules: `aphyn_ecs` (entities, resources, systems, runtime spawn and destroy),
//! `aphyn_transform` (local/global transforms, UI rects), `aphyn_render` (`create_mesh`,
//! materials, `Camera`, `CameraProjection`), `aphyn_render_features` (lit and UI material templates,
//! `MeshComponent`, `MaterialComponent`, `BatchedMeshComponent`, `StandardLightComponent`,
//! `CameraComponent`), `aphyn_asset_storage` (`AssetStorageResource`), `aphyn_ui` (`TextComponent`,
//! built-in font), `aphyn_app` (keyboard input, `TimeResource`), `aphyn_math` (`Quat`).

use aphyn::*;

mod camera;
mod house;
mod materials;
mod mesh;
mod physics;
mod rebuild;
mod scene;

aphyn_entry_point!(init_app);

pub fn init_app(ctx: AppInitCtx) {
    let AppInitCtx {
        mut data, mut behavior, ..
    } = ctx;

    let mut resources = data.resources_mut();
    resources.insert(physics::Sim::default());
    resources.insert(rebuild::Cubes::default());

    let mut start = behavior.as_mut();
    let mut start = start.get_mut(Schedule::Start);
    start.insert_system(scene::setup_scene);
    // the meshes and materials need the render module, the HUD font comes from the UI module
    start.order_group(SYSTEM_GROUP_RENDER).before_system(scene::setup_scene);
    start.order_group(SYSTEM_GROUP_UI).before_system(scene::setup_scene);

    let mut update = behavior.as_mut();
    let mut update = update.get_mut(Schedule::Update);
    update.insert_system(camera::orbit_camera);
    update.insert_system(rebuild::rebuild_house);
    update.insert_system(physics::simulate_voxels);
    update.insert_system(rebuild::write_cube_count);
    // new and moved cubes get their global transforms in the same frame
    update.order_system(camera::orbit_camera).before_group(SYSTEM_GROUP_TRANSFORM);
    update
        .order_system(rebuild::rebuild_house)
        .before_group(SYSTEM_GROUP_TRANSFORM);
    update
        .order_system(physics::simulate_voxels)
        .before_group(SYSTEM_GROUP_TRANSFORM);
    // the counter shows the new count in the frame of the rebuild
    update
        .order_system(rebuild::rebuild_house)
        .before_system(rebuild::write_cube_count);
    update.order_system(rebuild::write_cube_count).before_group(SYSTEM_GROUP_UI);

    scene::spawn_camera(data.as_mut());
}
