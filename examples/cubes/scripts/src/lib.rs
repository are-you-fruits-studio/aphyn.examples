//! Three vertex-colored cubes that spin and slide along the X, Y and Z axes.
//!
//! Shows the smallest complete scene: a camera entity, a mesh built in code with
//! `RenderApiResource::create_mesh`, the built-in unlit material template, and per-frame systems
//! that move entities through `LocalTransform`. Each cube is the child of a squashed parent, so
//! the parent's scale is inherited through the transform hierarchy.
//!
//! Engine modules: `aphyn_ecs` (entities, components, systems), `aphyn_transform` (local/global
//! transforms, parent/child), `aphyn_render` (mesh, material, camera), `aphyn_app` (`TimeResource`).

use aphyn::*;

aphyn_entry_point!(init_app);

pub fn init_app(ctx: AppInitCtx) {
    let AppInitCtx {
        mut data, mut behavior, ..
    } = ctx;

    let mut start = behavior.as_mut();
    let mut start = start.get_mut(Schedule::Start);
    start.insert_system(spawn_cubes);
    start.order_group(SYSTEM_GROUP_RENDER).before_system(spawn_cubes);

    let mut update = behavior.as_mut();
    let mut update = update.get_mut(Schedule::Update);
    update.insert_system(move_cubes);
    update.insert_system(rotate_cubes);

    spawn_camera(data.as_mut());
}

#[derive(Component)]
struct MovingCube;

#[derive(Component)]
struct RotatingCube;

fn spawn_camera(data: WorldDataMut) {
    let mut ec = data.into_entities_mut();

    let camera = ec.create_entity();
    ec.add_component(
        camera,
        GlobalTransform {
            scale_rotation: Mat::IDENTITY,
            position: Vec3::new(0.0, 0.0, -1.0),
        },
    )
    .ok()
    .unwrap();
    ec.add_component(
        camera,
        CameraComponent(Camera {
            near: 0.1,
            far: 1_000.0,
            projection: CameraProjection::Perspective {
                fov: 90_f32.to_radians(),
            },
        }),
    )
    .ok()
    .unwrap();
}

fn spawn_cubes(mut world: WorldDataMut) {
    let Some(render_api) = world.as_ref().resources().get::<RenderApiResource>() else {
        return;
    };

    let material =
        create_material_from_template_key(world.as_ref().resources(), BUILTIN_TEMPLATE_OPAQUE_UNLIT, MaterialData::new()).unwrap();

    let (vertices, indices) = cube_mesh(0.2);
    let mesh = render_api.create_mesh(&vertices, &indices, None);

    let material = world
        .as_mut()
        .resources_mut()
        .get_mut::<AssetStorageResource<Material>>()
        .unwrap()
        .insert(material);
    let mesh = world
        .as_mut()
        .resources_mut()
        .get_mut::<AssetStorageResource<StandardMesh>>()
        .unwrap()
        .insert(mesh);

    for _ in 0..3 {
        let mut parent_transform = LocalTransform::IDENTITY;
        parent_transform.scale.x *= 0.7;

        let mut ec = world.entities_mut();

        let parent = ec.create_entity();
        ec.add_component(parent, parent_transform).ok().unwrap();

        let cube = ec.create_entity();
        ec.add_component(cube, MeshComponent { mesh: mesh.clone() }).ok().unwrap();
        ec.add_component(
            cube,
            MaterialComponent {
                material: material.clone(),
            },
        )
        .ok()
        .unwrap();
        ec.add_component(cube, LocalTransform::IDENTITY).ok().unwrap();
        ec.add_component(cube, ChildComponent { parent }).ok().unwrap();
        ec.add_component(cube, RotatingCube).ok().unwrap();
        ec.add_component(cube, MovingCube).ok().unwrap();
    }
}

// one color per corner, flat normals
fn cube_mesh(half_size: f32) -> (Vec<StandardVertex>, Vec<u32>) {
    let corners = [
        ([0.0, 0.0, 0.0], [0.0, 0.0]),
        ([1.0, 0.0, 0.0], [1.0, 0.0]),
        ([0.0, 1.0, 0.0], [0.0, 1.0]),
        ([1.0, 1.0, 0.0], [1.0, 1.0]),
        ([0.0, 0.0, 1.0], [1.0, 0.0]),
        ([1.0, 0.0, 1.0], [0.0, 0.0]),
        ([0.0, 1.0, 1.0], [1.0, 1.0]),
        ([1.0, 1.0, 1.0], [0.0, 1.0]),
    ];
    let corners = corners.map(|(unit, uv): ([f32; 3], [f32; 2])| StandardVertex {
        position: unit.map(|v| (v * 2.0 - 1.0) * half_size),
        color: [unit[0], unit[1], unit[2], 1.0],
        uv,
        ..Default::default()
    });

    let triangles = [
        0, 1, 3, 0, 3, 2, 0, 4, 5, 0, 5, 1, 0, 6, 4, 0, 2, 6, 1, 7, 3, 1, 5, 7, 2, 7, 6, 2, 3, 7, 4, 6, 7, 4, 7, 5,
    ];

    // every triangle gets its own three vertices, so each face can have its own normal
    let mut vertices = triangles.iter().map(|&i| corners[i]).collect::<Vec<_>>();

    for [a, b, c] in vertices.as_chunks_mut::<3>().0 {
        let (pa, pb, pc) = (
            Vec3::from_array(a.position),
            Vec3::from_array(b.position),
            Vec3::from_array(c.position),
        );
        let normal = (pa - pc).cross(pa - pb).normalized().into_array();

        a.normal = normal;
        b.normal = normal;
        c.normal = normal;
    }

    let indices = (0..vertices.len() as u32).collect();

    (vertices, indices)
}

fn move_cubes(time: Res<TimeResource>, mut query: WorldQuery<(&mut LocalTransform, &MovingCube)>) {
    let offset = (time.frame_time as f32).cos() * 0.5;

    for (axis, (transform, _)) in query.iter_mut().enumerate() {
        match axis {
            0 => transform.position.x = offset,
            1 => transform.position.y = offset,
            2 => transform.position.z = offset,
            _ => {}
        }
    }
}

fn rotate_cubes(time: Res<TimeResource>, mut query: WorldQuery<(&mut LocalTransform, &RotatingCube)>) {
    for (i, (transform, _)) in query.iter_mut().enumerate() {
        let speed = (i + 1) as f64;

        transform.rotation = Quat::rotation_y(time.frame_time * speed) * Quat::rotation_z(-45.0_f64.to_radians());
    }
}
