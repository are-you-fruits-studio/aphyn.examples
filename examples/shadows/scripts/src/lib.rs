//! Shadows: orange cubes and pillars stand on a large floor under a slowly turning sun. The sun's
//! cascaded shadows sweep around every object, a warm spot light above a small table throws the
//! table's shadow into its circle of light, and a blue point light in front of the camera lights
//! the floor without a shadow.
//!
//! Shows turning shadows on per light with the `shadows` field of `StandardLightComponent`: a
//! directional light (cascaded shadow maps) rotated every frame, a spot light (one shadow map),
//! and a point light whose flag is ignored, because the engine does not render point light
//! shadows yet. A dim `AmbientLightResource` keeps the shadowed areas from going pure black. The
//! built-in lit template casts shadows with no extra setup; every object is the same cube mesh,
//! loaded from a Wavefront `.obj` through an `.asset` descriptor (`assets/cube.asset`) and scaled.
//!
//! Engine modules: `aphyn_render_features` (`StandardLightComponent`, `AmbientLightResource`, the
//! lit template, `create_material_from_template_key`, `MeshComponent`, `MaterialComponent`,
//! `CameraComponent`), `aphyn_render` (`Material`, `MaterialData`, `StandardMesh`, `Camera`),
//! `aphyn_asset_loading` and `aphyn_wavefront` (loading the cube), `aphyn_asset_storage`
//! (`AssetStorageResource`), `aphyn_transform` (`LocalTransform`), `aphyn_math` (`Vec3`, `Quat`),
//! `aphyn_app` (`TimeResource`), `aphyn_ecs`.

use aphyn::*;

aphyn_entry_point!(init_app);

pub fn init_app(ctx: AppInitCtx) {
    let AppInitCtx { mut behavior, .. } = ctx;

    let mut start = behavior.as_mut();
    let mut start = start.get_mut(Schedule::Start);
    start.insert_system(spawn_scene);
    start.order_group(SYSTEM_GROUP_ASSETS).before_system(spawn_scene);

    behavior.as_mut().get_mut(Schedule::Update).insert_system(rotate_sun);
}

#[derive(Component)]
struct Sun;

fn rotate_sun(time: Res<TimeResource>, mut query: WorldQuery<&mut LocalTransform, WithFilter<Sun>>) {
    let angle = time.frame_time * 0.2;

    for transform in query.iter_mut() {
        transform.rotation = Quat::rotation_y(angle) * Quat::rotation_x(-0.6);
    }
}

fn spawn_scene(mut world: WorldDataMut) {
    let (mut res, mut ec, _) = world.as_tuple_mut();

    res.get_mut::<AmbientLightResource>().unwrap().color = Vec3::splat(0.05);

    let mut ground_data = MaterialData::new();
    ground_data.insert("roughness", MaterialDataValue::F32(0.9));
    let ground_material = create_material_from_template_key(res.as_ref(), BUILTIN_TEMPLATE_OPAQUE_LIT, ground_data).unwrap();

    let mut cube_data = MaterialData::new();
    cube_data.insert("color", MaterialDataValue::Vec4(Vec4::new(0.9, 0.5, 0.3, 1.0)));
    cube_data.insert("roughness", MaterialDataValue::F32(0.6));
    let cube_material = create_material_from_template_key(res.as_ref(), BUILTIN_TEMPLATE_OPAQUE_LIT, cube_data).unwrap();

    let materials = res.get_mut::<AssetStorageResource<Material>>().unwrap();
    let ground_material = materials.insert(ground_material);
    let cube_material = materials.insert(cube_material);

    let meshes = res.get_mut::<AssetStorageResource<StandardMesh>>().unwrap();
    let cube_mesh = meshes.get_registered("cube.asset").unwrap().clone();

    let camera = ec.create_entity();
    ec.set_components(
        camera,
        (
            LocalTransform {
                position: Vec3::new(0.0, 9.0, -16.0),
                rotation: Quat::rotation_x(30.0_f64.to_radians()),
                ..Default::default()
            },
            CameraComponent(Camera {
                near: 0.1,
                far: 1_000.0,
                projection: CameraProjection::Perspective {
                    fov: 70_f32.to_radians(),
                },
            }),
        ),
    );

    // the cube mesh spans -1..1, so `scale` is half the size and `position` is the center
    let mut spawn_cube = |position: Vec3<f32>, scale: Vec3<f32>, material: &AssetHandle<Material>| {
        let cube = ec.create_entity();
        ec.set_components(
            cube,
            (
                LocalTransform {
                    position,
                    scale,
                    ..Default::default()
                },
                MaterialComponent {
                    material: material.clone(),
                },
                MeshComponent { mesh: cube_mesh.clone() },
            ),
        );
    };

    // the floor's top face is at y = 0
    spawn_cube(Vec3::new(0.0, -1.0, 0.0), Vec3::new(30.0, 1.0, 30.0), &ground_material);
    spawn_cube(Vec3::new(0.0, 1.0, 0.0), Vec3::splat(1.0), &cube_material);
    spawn_cube(Vec3::new(-4.0, 3.0, 3.0), Vec3::new(0.5, 3.0, 0.5), &cube_material);
    spawn_cube(Vec3::new(4.0, 0.5, -2.0), Vec3::splat(0.5), &cube_material);

    // a table: a slab on one leg, right under the spot light
    spawn_cube(Vec3::new(7.0, 2.5, 6.0), Vec3::new(1.5, 0.25, 1.5), &cube_material);
    spawn_cube(Vec3::new(7.0, 1.0, 6.0), Vec3::new(0.25, 1.0, 0.25), &cube_material);

    // a row of pillars going away from the camera, across the shadow cascades
    for i in 0..8 {
        let z = 8.0 + i as f32 * 4.0;
        spawn_cube(Vec3::new(-8.0, 1.5, z), Vec3::new(0.5, 1.5, 0.5), &cube_material);
    }

    // rotate_sun sets the rotation every frame
    let sun = ec.create_entity();
    ec.set_components(
        sun,
        (
            LocalTransform::default(),
            StandardLightComponent::Directional {
                color: Vec3::splat(2.5),
                shadows: true,
            },
            Sun,
        ),
    );

    // with no rotation, a spot light points straight down
    let spot = ec.create_entity();
    ec.set_components(
        spot,
        (
            LocalTransform {
                position: Vec3::new(7.0, 6.0, 6.0),
                ..Default::default()
            },
            StandardLightComponent::Spot {
                color: Vec3::new(1.0, 0.9, 0.6) * 60.0_f32,
                range: 20.0,
                fov: 70_f32.to_radians(),
                shadows: true,
            },
        ),
    );

    // point light shadows are not supported yet, the flag is ignored
    let point = ec.create_entity();
    ec.set_components(
        point,
        (
            LocalTransform {
                position: Vec3::new(-5.0, 2.0, -4.0),
                ..Default::default()
            },
            StandardLightComponent::Point {
                color: Vec3::new(0.3, 0.5, 1.0) * 10.0_f32,
                range: 20.0,
                shadows: true,
            },
        ),
    );
}
