//! Text in world space: a line of bitmap-font text in front of the camera that shows the seconds
//! since start, rewritten every frame.
//!
//! Shows `TextComponent` with a built-in font, the material that draws it (the opaque unlit
//! template with the font atlas as `color_tex`), and updating the text in place. The FPS counter
//! module prints the frame rate to the console.
//!
//! Engine modules: `aphyn_ui` (`TextComponent`, built-in fonts in `StandardUiAssetsResource`),
//! `aphyn_render_features` (opaque unlit material template, `MaterialComponent`,
//! `CameraComponent`), `aphyn_modules` (FPS counter), `aphyn_app` (`TimeResource`),
//! `aphyn_transform`, `aphyn_ecs`.

use std::fmt::Write;

use aphyn::*;

aphyn_entry_point!(init_app);

pub fn init_app(ctx: AppInitCtx) {
    let AppInitCtx {
        mut data,
        mut behavior,
        render,
        ..
    } = ctx;

    fps_counter::add_module_to(data.as_mut(), behavior.as_mut(), render);

    let mut start = behavior.as_mut();
    let mut start = start.get_mut(Schedule::Start);
    start.insert_system(spawn_text);
    // the material needs the render module, the font comes from the UI module
    start.order_group(SYSTEM_GROUP_RENDER).before_system(spawn_text);
    start.order_group(SYSTEM_GROUP_UI).before_system(spawn_text);

    let mut update = behavior.as_mut();
    let mut update = update.get_mut(Schedule::Update);
    update.insert_system(write_time_into_text);
    update.order_system(write_time_into_text).before_group(SYSTEM_GROUP_UI);

    let mut ec = data.as_mut().into_entities_mut();
    let camera = ec.create_entity();
    ec.set_components(
        camera,
        (
            LocalTransform {
                position: Vec3::new(0.0, 0.0, -1.0),
                ..LocalTransform::IDENTITY
            },
            CameraComponent(Camera {
                near: 0.1,
                far: 1_000.0,
                projection: CameraProjection::Perspective {
                    fov: 90_f32.to_radians(),
                },
            }),
        ),
    );
}

fn spawn_text(mut world: WorldDataMut) {
    let ui_assets = world.as_ref().resources().get::<StandardUiAssetsResource>().unwrap();
    let atlas = ui_assets.texture_text_px_5_7.clone();
    let font = ui_assets.font_px_5_7.clone();

    let mut material_data = MaterialData::new();
    material_data.insert("color_tex", MaterialDataValue::Texture(atlas));
    let material =
        create_material_from_template_key(world.as_ref().resources(), BUILTIN_TEMPLATE_OPAQUE_UNLIT, material_data).unwrap();
    let material = world
        .as_mut()
        .resources_mut()
        .get_mut::<AssetStorageResource<Material>>()
        .unwrap()
        .insert(material);

    let mut ec = world.entities_mut();
    let text = ec.create_entity();
    ec.set_components(
        text,
        (
            LocalTransform::IDENTITY,
            BatchedMeshComponent::default(),
            MaterialComponent { material },
            TextComponent {
                font,
                // world units, since this text is not part of the UI
                font_size: UiVal::px(0.4),
                text: "0.00".into(),
                horizontal_align: HorizontalAlign::Middle,
                vertical_align: VerticalAlign::Middle,
                is_y_inverted: false,
                horizontal_spacing: UiVal::px(0.2),
                color: Vec4::splat(1.0),
            },
        ),
    );
}

fn write_time_into_text(time: Res<TimeResource>, mut texts: WorldQuery<&mut TextComponent>) {
    for text in texts.iter_mut() {
        text.text.clear();
        write!(text.text, "{:.2}", time.frame_time).unwrap();
    }
}
