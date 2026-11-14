use aphyn::*;

use crate::camera::{CameraOrbit, orbit_transform};
use crate::house::spawn_house;
use crate::materials::Materials;
use crate::mesh::upload_cube;
use crate::rebuild::{CountText, Cubes, DEFAULT_LEVEL, HouseAssets};

const HINT: &str = "SPACE / ENTER: explode     TAB / R: reset     1-9: cube level";

pub fn spawn_camera(mut data: WorldDataMut) {
    let mut ec = data.entities_mut();
    let camera = ec.create_entity();
    ec.set_components(
        camera,
        (
            orbit_transform(0.0),
            CameraComponent(Camera {
                near: 0.1,
                far: 500.0,
                projection: CameraProjection::Perspective {
                    fov: 55_f32.to_radians(),
                },
            }),
            CameraOrbit { angle: 0.0 },
        ),
    );
}

pub fn setup_scene(mut world: WorldDataMut) {
    let materials = Materials::create(&mut world);
    let cube = upload_cube(&mut world);

    spawn_lights(&mut world);

    // ground slab, its top surface at y = 0
    let mut ec = world.entities_mut();
    let ground = ec.create_entity();
    ec.set_components(
        ground,
        (
            LocalTransform {
                position: Vec3::new(0.0, -0.25, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::new(200.0, 0.5, 200.0),
            },
            MeshComponent { mesh: cube.clone() },
            MaterialComponent {
                material: materials.ground.clone(),
            },
        ),
    );

    let ids = spawn_house(&mut world, &cube, &materials, DEFAULT_LEVEL);
    *world.resources_mut().get_mut::<Cubes>().unwrap() = Cubes {
        level: DEFAULT_LEVEL,
        ids,
    };

    // the hint is sized from the window height and fits windows at least 4:3 wide
    spawn_text(&mut world, &materials, 0.94, 0.03, HINT);
    let count = spawn_text(&mut world, &materials, 0.06, 0.04, "");
    world.entities_mut().add_component(count, CountText).ok().unwrap();

    world.resources_mut().insert(HouseAssets { cube, materials });
}

// A strong key light and a weak fill from the other side, so faces away from the key light are not
// pure black. Directional lights shine along their local -Y axis.
fn spawn_lights(world: &mut WorldDataMut) {
    let mut ec = world.entities_mut();
    for (yaw, pitch, strength) in [(-30.0_f64, -55.0_f64, 1.5_f32), (150.0, -35.0, 0.45)] {
        let light = ec.create_entity();
        ec.set_components(
            light,
            (
                LocalTransform {
                    rotation: Quat::rotation_y(yaw.to_radians()) * Quat::rotation_x(pitch.to_radians()),
                    ..LocalTransform::IDENTITY
                },
                StandardLightComponent::Directional {
                    color: Vec3::splat(strength),
                },
            ),
        );
    }
}

// A line of HUD text centered at `y`; `y` and `height` are fractions of the window height, `y` from the top.
fn spawn_text(world: &mut WorldDataMut, materials: &Materials, y: f32, height: f32, text: &str) -> EntityId {
    let font = world
        .resources()
        .get::<StandardUiAssetsResource>()
        .unwrap()
        .font_px_8_12
        .clone();
    let mut ec = world.entities_mut();
    let entity = ec.create_entity();
    ec.set_components(
        entity,
        (
            LocalRectComponent {
                anchor: Vec2::new(0.5, y),
                pivot: Vec2::splat(0.5),
                scale: Vec2::new(UiVal::vw(0.9).into(), UiVal::vh(height).into()),
                z: -10.0,
                ..Default::default()
            },
            BatchedMeshComponent::default(),
            MaterialComponent {
                material: materials.hud_text.clone(),
            },
            TextComponent {
                font,
                text: text.into(),
                font_size: UiVal::ph(1.0),
                horizontal_align: HorizontalAlign::Middle,
                ..Default::default()
            },
        ),
    );
    entity
}
