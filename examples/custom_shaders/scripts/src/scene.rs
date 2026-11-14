use std::f32::consts::PI;

use aphyn::*;

use crate::mesh;
use crate::update::{DemoState, OrbitingLight, Spin, StatusText};

const CAMERA_DISTANCE: f32 = 8.5;
const CAMERA_FOV_DEGREES: f32 = 40.0;

const CELL_SPACING_X: f32 = 2.5;
const CELL_SPACING_Y: f32 = 2.6;
const LABEL_OFFSET_Y: f32 = 1.02;
const LABEL_SIZE: f32 = 0.16;
const LABEL_COLOR: Vec4<f32> = Vec4::new(0.75, 0.82, 0.9, 1.0);
const SPHERE_SCALE: f32 = 0.85;

const WAVE_TILT_RADIANS: f64 = -0.95;
const WAVE_TILE_SIZE: f32 = 0.85;
// Distance of a tile center from the cell center, in tile sizes. Above 0.5 it leaves a gap, since
// neighboring tiles are displaced with different phases and their edges do not meet.
const WAVE_TILE_OFFSET: f32 = 0.56;

// material colors are linear
const PARITY_COLOR: Vec4<f32> = Vec4::new(0.612, 0.047, 0.029, 1.0);
const CORE_COLOR: Vec4<f32> = Vec4::new(0.029, 0.531, 0.099, 1.0);

struct TextStyle {
    font: AssetHandle<Font>,
    material: AssetHandle<Material>,
}

// Four cells in the top row and three in the bottom one, each row centered.
fn cell_center(column: usize, row: usize) -> Vec3<f32> {
    let columns = if row == 0 { 4.0 } else { 3.0 };

    Vec3::new(
        (column as f32 - (columns - 1.0) * 0.5) * CELL_SPACING_X,
        (0.5 - row as f32) * CELL_SPACING_Y,
        0.0,
    )
}

pub fn spawn_camera(data: WorldDataMut) {
    let mut ec = data.into_entities_mut();

    let camera = ec.create_entity();
    ec.set_components(
        camera,
        (
            LocalTransform {
                position: Vec3::new(0.0, 0.0, -CAMERA_DISTANCE),
                ..LocalTransform::IDENTITY
            },
            CameraComponent(Camera {
                near: 0.1,
                far: 100.0,
                projection: CameraProjection::Perspective {
                    fov: CAMERA_FOV_DEGREES.to_radians(),
                },
            }),
        ),
    );
}

pub fn setup_scene(mut world: WorldDataMut) {
    world.as_mut().resources_mut().get_mut::<BloomResource>().unwrap().is_enabled = true;

    spawn_lights(&mut world);

    let sphere = mesh::upload(&mut world, mesh::make_sphere(0.0, 2.0 * PI, 48, 32));
    let right_half = mesh::upload(&mut world, mesh::make_sphere(0.0, PI, 24, 32));
    let left_half = mesh::upload(&mut world, mesh::make_sphere(PI, 2.0 * PI, 24, 32));
    let plane = mesh::upload(&mut world, mesh::make_plane(64));

    let style = text_style(&mut world);
    let mut custom_materials = Vec::new();

    // Top row: one debug shader drawn through three materials, then the parity sphere.
    for (column, (key, label)) in [
        ("materials/debug_normal.asset", "debug: normal"),
        ("materials/debug_uv.asset", "debug: uv"),
        ("materials/debug_tangent.asset", "debug: tangent"),
    ]
    .into_iter()
    .enumerate()
    {
        let material = registered_material(&world, key);
        spawn_sphere(&mut world, &sphere, &material, cell_center(column, 0), 1.0);
        spawn_label(&mut world, &style, column, 0, label);
        custom_materials.push(material);
    }

    let (parity_engine, parity_custom) = parity_materials(&mut world);
    spawn_sphere(&mut world, &left_half, &parity_engine, cell_center(3, 0), 1.0);
    spawn_sphere(&mut world, &right_half, &parity_custom, cell_center(3, 0), 1.0);
    spawn_label(&mut world, &style, 3, 0, "parity: engine|custom");
    custom_materials.push(parity_custom);

    // Bottom row.
    let toon = registered_material(&world, "materials/toon.asset");
    spawn_sphere(&mut world, &sphere, &toon, cell_center(0, 1), 1.0);
    spawn_label(&mut world, &style, 0, 1, "toon + rim");
    custom_materials.push(toon);

    let waves = registered_material(&world, "materials/waves.asset");
    spawn_wave_tiles(&mut world, &plane, &waves, cell_center(1, 1));
    spawn_label(&mut world, &style, 1, 1, "waves: 4 instances");
    custom_materials.push(waves);

    let dissolve = registered_material(&world, "materials/dissolve.asset");
    let mut core_data = MaterialData::new();
    core_data.insert("color", MaterialDataValue::Vec4(CORE_COLOR));
    let core = create_material_from_template_key(world.as_ref().resources(), BUILTIN_TEMPLATE_OPAQUE_UNLIT, core_data).unwrap();
    let core = insert_material(&mut world, core);
    let dissolving = spawn_sphere(&mut world, &sphere, &dissolve, cell_center(2, 1), 1.0);
    world.entities_mut().add_component(dissolving, Spin(0.5)).ok().unwrap();
    spawn_sphere(&mut world, &sphere, &core, cell_center(2, 1), 0.45);
    spawn_label(&mut world, &style, 2, 1, "dissolve + bloom");
    custom_materials.push(dissolve);

    spawn_status_line(&mut world);

    world
        .as_mut()
        .resources_mut()
        .get_mut::<DemoState>()
        .unwrap()
        .custom_materials = custom_materials;
}

fn spawn_lights(world: &mut WorldDataMut) {
    let mut ec = world.entities_mut();

    // A point light's color is divided by the squared distance, 80 to 120 from here to the cells.
    let fill = ec.create_entity();
    ec.set_components(
        fill,
        (
            LocalTransform {
                position: Vec3::new(0.0, 3.0, -9.0),
                ..LocalTransform::IDENTITY
            },
            StandardLightComponent::Point {
                color: Vec3::splat(55.0),
                range: 100.0,
            },
        ),
    );

    let orbiting = ec.create_entity();
    ec.set_components(
        orbiting,
        (
            LocalTransform::IDENTITY,
            StandardLightComponent::Point {
                color: Vec3::new(30.0, 24.0, 16.0),
                range: 100.0,
            },
            OrbitingLight,
        ),
    );
}

// The same data drawn by the engine's lit template and by the engine's standard template wrapped
// around the custom copy of the lit shader.
fn parity_materials(world: &mut WorldDataMut) -> (AssetHandle<Material>, AssetHandle<Material>) {
    let data = || {
        let mut data = MaterialData::new();
        data.insert("color", MaterialDataValue::Vec4(PARITY_COLOR));
        data.insert("metallic", MaterialDataValue::F32(0.3));
        data.insert("roughness", MaterialDataValue::F32(0.35));
        data
    };

    let engine = create_material_from_template_key(world.as_ref().resources(), BUILTIN_TEMPLATE_OPAQUE_LIT, data()).unwrap();

    let custom = {
        let mut world_mut = world.as_mut();
        let mut res = world_mut.resources_mut();
        let shader = res
            .get::<AssetStorageResource<Shader>>()
            .unwrap()
            .get_registered("shaders/lit_copy.asset")
            .cloned()
            .expect("shaders/lit_copy.asset not loaded");
        let template = res
            .get_mut::<AssetStorageResource<MaterialTemplate>>()
            .unwrap()
            .insert(standard_material_template(shader, false));

        let render_api = res.get::<RenderApiResource>().unwrap();
        let templates = res.get::<AssetStorageResource<MaterialTemplate>>().unwrap();
        let shaders = res.get::<AssetStorageResource<Shader>>().unwrap();
        render_api.create_material(templates, shaders, template, data())
    };

    (insert_material(world, engine), insert_material(world, custom))
}

fn spawn_wave_tiles(
    world: &mut WorldDataMut,
    mesh: &AssetHandle<StandardMesh>,
    material: &AssetHandle<Material>,
    center: Vec3<f32>,
) {
    let tilt = Quat::rotation_x(WAVE_TILT_RADIANS);
    let mut ec = world.entities_mut();

    let d = WAVE_TILE_OFFSET;
    for (x, z) in [(-d, -d), (d, -d), (-d, d), (d, d)] {
        let offset = tilt.to_matrix() * Vec3::new(x, 0.0, z) * WAVE_TILE_SIZE;

        let tile = ec.create_entity();
        ec.set_components(
            tile,
            (
                LocalTransform {
                    position: center + offset,
                    rotation: tilt,
                    scale: Vec3::splat(WAVE_TILE_SIZE),
                },
                MeshComponent { mesh: mesh.clone() },
                MaterialComponent {
                    material: material.clone(),
                },
            ),
        );
    }
}

fn spawn_sphere(
    world: &mut WorldDataMut,
    mesh: &AssetHandle<StandardMesh>,
    material: &AssetHandle<Material>,
    position: Vec3<f32>,
    scale: f32,
) -> EntityId {
    let mut ec = world.entities_mut();

    let entity = ec.create_entity();
    ec.set_components(
        entity,
        (
            LocalTransform {
                position,
                rotation: Quat::IDENTITY,
                scale: Vec3::splat(scale * SPHERE_SCALE),
            },
            MeshComponent { mesh: mesh.clone() },
            MaterialComponent {
                material: material.clone(),
            },
        ),
    );

    entity
}

fn text_style(world: &mut WorldDataMut) -> TextStyle {
    let ui_assets = world.as_ref().resources().get::<StandardUiAssetsResource>().unwrap();
    let atlas = ui_assets.texture_text_px_8_12.clone();
    let font = ui_assets.font_px_8_12.clone();

    let mut data = MaterialData::new();
    data.insert("color_tex", MaterialDataValue::Texture(atlas));
    let material = create_material_from_template_key(world.as_ref().resources(), BUILTIN_TEMPLATE_OPAQUE_UNLIT, data).unwrap();

    TextStyle {
        font,
        material: insert_material(world, material),
    }
}

// World-space text in front of the camera; sizes are in world units.
fn spawn_label(world: &mut WorldDataMut, style: &TextStyle, column: usize, row: usize, text: &str) {
    let mut ec = world.entities_mut();

    let entity = ec.create_entity();
    ec.set_components(
        entity,
        (
            LocalTransform {
                position: cell_center(column, row) - Vec3::new(0.0, LABEL_OFFSET_Y, 0.0),
                ..LocalTransform::IDENTITY
            },
            BatchedMeshComponent::default(),
            MaterialComponent {
                material: style.material.clone(),
            },
            TextComponent {
                font: style.font.clone(),
                font_size: UiVal::px(LABEL_SIZE),
                text: text.into(),
                horizontal_align: HorizontalAlign::Middle,
                vertical_align: VerticalAlign::Middle,
                is_y_inverted: false,
                horizontal_spacing: UiVal::px(0.0),
                color: LABEL_COLOR,
            },
        ),
    );
}

// Screen-space text: an 18 px high rect at the top center of the window, drawn by a UI material.
fn spawn_status_line(world: &mut WorldDataMut) {
    let ui_assets = world.as_ref().resources().get::<StandardUiAssetsResource>().unwrap();
    let font = ui_assets.font_px_8_12.clone();
    let mut data = MaterialData::new();
    data.insert(
        "color_tex",
        MaterialDataValue::Texture(ui_assets.texture_text_px_8_12.clone()),
    );
    let material = create_material_from_template_key(world.as_ref().resources(), BUILTIN_TEMPLATE_UI_OPAQUE_UNLIT, data).unwrap();
    let material = insert_material(world, material);

    let mut ec = world.entities_mut();
    let entity = ec.create_entity();
    ec.set_components(
        entity,
        (
            LocalRectComponent {
                anchor: Vec2::new(0.5, 0.0),
                pivot: Vec2::new(0.5, 0.0),
                offset: Vec2::new(UiVal::px(0.0), UiVal::px(12.0)),
                scale: Vec2::new(UiVal::vw(1.0), UiVal::px(18.0)).map(Into::into),
                ..Default::default()
            },
            BatchedMeshComponent::default(),
            MaterialComponent { material },
            TextComponent {
                font,
                font_size: UiVal::ph(1.0),
                horizontal_align: HorizontalAlign::Middle,
                color: Vec4::new(0.9, 0.95, 1.0, 1.0),
                ..Default::default()
            },
            StatusText,
        ),
    );
}

fn registered_material(world: &WorldDataMut, key: &str) -> AssetHandle<Material> {
    world
        .as_ref()
        .resources()
        .get::<AssetStorageResource<Material>>()
        .unwrap()
        .get_registered(key)
        .cloned()
        .unwrap_or_else(|| panic!("{key} not loaded"))
}

fn insert_material(world: &mut WorldDataMut, material: Material) -> AssetHandle<Material> {
    world
        .as_mut()
        .resources_mut()
        .get_mut::<AssetStorageResource<Material>>()
        .unwrap()
        .insert(material)
}
