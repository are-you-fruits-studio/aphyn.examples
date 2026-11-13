//! Bloom: a small white cube floats just off the corner of a large floor and carries a strong
//! point light, so most of the floor is blown out far brighter than white. The bloom
//! post-processing pass spreads those HDR pixels into a soft glow along the floor's edges and
//! around the cube.
//!
//! Shows turning on bloom through `BloomResource`, a point light bright enough to push a lit
//! surface past the bloom threshold, an unlit cube on the built-in unlit template, a mesh loaded from a Wavefront `.obj` through an `.asset` descriptor
//! (`assets/cube.asset`), and a camera placed as the child of a rotated pivot entity.
//!
//! Engine modules: `aphyn_render_features` (`BloomResource`, lit and unlit material templates,
//! `MeshComponent`, `StandardLightComponent`, `CameraComponent`), `aphyn_asset_loading` and
//! `aphyn_wavefront` (loading the cube), `aphyn_asset_storage` (`AssetStorageResource`),
//! `aphyn_transform` (parent/child), `aphyn_ecs`.

use aphyn::*;

aphyn_entry_point!(init_app);

pub fn init_app(ctx: AppInitCtx) {
    let AppInitCtx { mut behavior, .. } = ctx;

    let mut start = behavior.as_mut();
    let mut start = start.get_mut(Schedule::Start);
    start.insert_system(spawn_scene);
    start.order_group(SYSTEM_GROUP_ASSETS).before_system(spawn_scene);
}

fn spawn_scene(mut world: WorldDataMut) {
    let (mut res, mut ec, _) = world.as_tuple_mut();

    let bloom = res.get_mut::<BloomResource>().unwrap();
    bloom.is_enabled = true;
    bloom.intensity = 2.0;

    let mut ground_data = MaterialData::new();
    ground_data.insert("roughness", MaterialDataValue::F32(1.0));
    let ground_material = create_material_from_template_key(res.as_ref(), BUILTIN_TEMPLATE_OPAQUE_LIT, ground_data).unwrap();

    // plain white: the unlit template draws the vertex and material color only, no emission
    let light_material =
        create_material_from_template_key(res.as_ref(), BUILTIN_TEMPLATE_OPAQUE_UNLIT, MaterialData::new()).unwrap();

    let materials = res.get_mut::<AssetStorageResource<Material>>().unwrap();
    let ground_material = materials.insert(ground_material);
    let light_material = materials.insert(light_material);

    let meshes = res.get_mut::<AssetStorageResource<StandardMesh>>().unwrap();
    let cube_mesh = meshes.get_registered("cube.asset").unwrap().clone();

    // the camera sits 10 units behind a pivot, and the pivot's rotation aims it
    let camera_pivot = ec.create_entity();
    let camera = ec.create_entity();

    ec.set_components(
        camera_pivot,
        (LocalTransform {
            rotation: Quat::rotation_y(-30.0_f64.to_radians()) * Quat::rotation_x(7.0_f64.to_radians()),
            ..Default::default()
        },),
    );
    ec.set_components(
        camera,
        (
            LocalTransform {
                position: Vec3::new(0.0, 0.0, -10.0),
                ..Default::default()
            },
            ChildComponent { parent: camera_pivot },
            CameraComponent(Camera {
                near: 0.1,
                far: 1_000.0,
                projection: CameraProjection::Perspective {
                    fov: 90_f32.to_radians(),
                },
            }),
        ),
    );

    let light = ec.create_entity();
    ec.set_components(
        light,
        (
            LocalTransform {
                position: Vec3::new(-1.0, 0.5, -1.0),
                scale: Vec3::splat(0.1),
                ..Default::default()
            },
            StandardLightComponent::Point {
                color: Vec3::splat(3_000.0),
                range: 1_000.0,
            },
            MaterialComponent {
                material: light_material,
            },
            MeshComponent { mesh: cube_mesh.clone() },
        ),
    );

    // a 200-unit cube: its top face is the floor at y = 0, with a corner at the origin
    let ground = ec.create_entity();
    ec.set_components(
        ground,
        (
            LocalTransform {
                position: Vec3::new(100.0, -100.0, 100.0),
                scale: Vec3::splat(100.0),
                ..Default::default()
            },
            MaterialComponent {
                material: ground_material,
            },
            MeshComponent { mesh: cube_mesh },
        ),
    );
}
