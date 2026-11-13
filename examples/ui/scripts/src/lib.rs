//! Retained UI built from rect entities: a 200 px panel in the centre of the window that masks its
//! children, a grey box and a magenta label inside it, a column of three colored squares laid out
//! by `RectChildAlignComponent`, and a small square that follows the mouse with damping.
//!
//! Shows `LocalRectComponent` sizing in pixels and in fractions of the parent and the window
//! (`UiVal::px`, `pw`, `ph`, `vmin`, `vh`, `vd`), parent/child rect hierarchies,
//! `ChildrenRectMaskComponent`, `ImageComponent` colors drawn with one white UI material,
//! `TextComponent` with the UI material and a font atlas, and moving a rect from a system.
//!
//! Engine modules: `aphyn_ui` (rects, layout, images, text), `aphyn_render_features` (UI material
//! template, `MaterialComponent`, `CameraComponent`), `aphyn_transform` (rect transforms),
//! `aphyn_modules` (FPS counter), `aphyn_app` (input), `aphyn_math` (`damp`), `aphyn_ecs`.

use aphyn::*;

const CURSOR_SMOOTH_TIME: f32 = 5.0;
const CURSOR_DAMPING: f32 = 0.1;

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
    start.insert_system(spawn_ui);
    // the materials need the render module, the font comes from the UI module
    start.order_group(SYSTEM_GROUP_RENDER).before_system(spawn_ui);
    start.order_group(SYSTEM_GROUP_UI).before_system(spawn_ui);

    let mut update = behavior.as_mut();
    let mut update = update.get_mut(Schedule::Update);
    update.insert_system(follow_cursor);
    update.order_system(follow_cursor).before_group(SYSTEM_GROUP_TRANSFORM);

    // nothing renders without a camera, even when everything on screen is UI
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

#[derive(Component, Default)]
struct Cursor {
    velocity: Vec2<f32>,
}

fn spawn_ui(mut world: WorldDataMut) {
    let ui_assets = world.as_ref().resources().get::<StandardUiAssetsResource>().unwrap();
    let atlas = ui_assets.texture_text_px_8_12.clone();
    let font = ui_assets.font_px_8_12.clone();

    // one white material for every colored rect; ImageComponent.color tints it per rect
    let white = create_material_from_template_key(
        world.as_ref().resources(),
        BUILTIN_TEMPLATE_UI_OPAQUE_UNLIT,
        MaterialData::new(),
    )
    .unwrap();
    let mut text_data = MaterialData::new();
    text_data.insert("color_tex", MaterialDataValue::Texture(atlas));
    let text = create_material_from_template_key(world.as_ref().resources(), BUILTIN_TEMPLATE_UI_OPAQUE_UNLIT, text_data).unwrap();

    let mut resources = world.resources_mut();
    let materials = resources.get_mut::<AssetStorageResource<Material>>().unwrap();
    let white = materials.insert(white);
    let text = materials.insert(text);

    let mut ec = world.entities_mut();

    // A centered container with no size of its own, a panel 100 px larger than it on every side
    // that clips its children, and inside the panel a grey box with a label.
    let container = spawn_rect(
        &mut ec,
        &white,
        LocalRectComponent {
            parent_padding_min: Vec2::new(UiVal::pw(0.5), UiVal::ph(0.5)),
            parent_padding_max: Vec2::new(UiVal::pw(0.5), UiVal::ph(0.5)),
            z: 0.0,
            ..Default::default()
        },
    );
    let panel = spawn_rect(
        &mut ec,
        &white,
        LocalRectComponent {
            parent_padding_min: Vec2::new(UiVal::px(-100.0), UiVal::px(-100.0)),
            parent_padding_max: Vec2::new(UiVal::px(-100.0), UiVal::px(-100.0)),
            z: -10.0,
            ..Default::default()
        },
    );
    ec.set_components(
        panel,
        (
            ImageComponent::default(),
            ChildrenRectMaskComponent,
            ChildComponent { parent: container },
        ),
    );
    let grey_box = spawn_rect(
        &mut ec,
        &white,
        LocalRectComponent {
            parent_padding_min: Vec2::new(UiVal::px(10.0), UiVal::px(10.0)),
            parent_padding_max: Vec2::new(UiVal::vmin(0.1), UiVal::vmin(0.1)),
            z: -10.0,
            ..Default::default()
        },
    );
    ec.set_components(
        grey_box,
        (
            ImageComponent {
                color: Vec4::splat(0.6),
                ..Default::default()
            },
            ChildComponent { parent: panel },
        ),
    );
    let label = spawn_rect(
        &mut ec,
        &text,
        LocalRectComponent {
            z: -10.0,
            ..Default::default()
        },
    );
    ec.set_components(
        label,
        (
            TextComponent {
                font,
                // small enough to read inside the 200 px panel, which still clips the first letter
                font_size: UiVal::px(16.0),
                text: "Hello from Aphyn".into(),
                horizontal_align: HorizontalAlign::Middle,
                vertical_align: VerticalAlign::Bottom,
                // UI space is y-down
                is_y_inverted: true,
                horizontal_spacing: UiVal::px(0.0),
                color: Vec4::new(1.0, 0.0, 1.0, 1.0),
            },
            ChildComponent { parent: grey_box },
        ),
    );

    // A white column in the top-left corner, sized by its children, stacking three squares.
    let column = spawn_rect(
        &mut ec,
        &white,
        LocalRectComponent {
            pivot: Vec2::splat(0.0),
            anchor: Vec2::splat(0.0),
            scale: Vec2::new(None.into(), None.into()),
            ..Default::default()
        },
    );
    ec.set_components(
        column,
        (
            ImageComponent {
                color: Vec4::splat(1.0),
                ..Default::default()
            },
            RectChildAlignComponent {
                direction: UiDirection::Vertical,
                anchor: Vec2::new(0.5, 0.5),
                min_gap: UiVal::vd(0.1),
                spacing: UiSpacing::Chunk,
            },
        ),
    );
    let squares = [
        (50.0, Vec4::new(1.0, 0.0, 0.0, 1.0)),
        (40.0, Vec4::new(0.0, 1.0, 0.0, 1.0)),
        (70.0, Vec4::new(0.0, 0.0, 1.0, 1.0)),
    ]
    .map(|(size, color)| {
        let square = spawn_rect(
            &mut ec,
            &white,
            LocalRectComponent {
                scale: Vec2::splat(UiVal::px(size).into()),
                ..Default::default()
            },
        );
        ec.add_component(
            square,
            ImageComponent {
                color,
                ..Default::default()
            },
        )
        .ok()
        .unwrap();
        square
    });
    link(&mut ec, column, &squares);

    // A 3 px square that follows the mouse.
    let cursor = spawn_rect(
        &mut ec,
        &white,
        LocalRectComponent {
            anchor: Vec2::splat(0.0),
            pivot: Vec2::splat(0.0),
            scale: Vec2::splat(UiVal::px(3.0).into()),
            ..Default::default()
        },
    );
    ec.set_components(
        cursor,
        (
            ImageComponent {
                color: Vec4::splat(1.0),
                ..Default::default()
            },
            Cursor::default(),
        ),
    );
}

// A rect drawn with `material`: the components every UI element here has.
fn spawn_rect(ec: &mut EntitiesHolderMut, material: &AssetHandle<Material>, rect: LocalRectComponent) -> EntityId {
    let entity = ec.create_entity();
    ec.set_components(
        entity,
        (
            rect,
            BatchedMeshComponent::default(),
            MaterialComponent {
                material: material.clone(),
            },
        ),
    );
    entity
}

// A child's `ChildComponent` is enough to parent it: the engine fills the parent's `ParentComponent`
// from those links, but in query order. `RectChildAlignComponent` stacks the children in list order,
// so the column lists them itself; the engine keeps a list it finds and only appends missing children.
fn link(ec: &mut EntitiesHolderMut, parent: EntityId, children: &[EntityId]) {
    ec.add_component(
        parent,
        ParentComponent {
            children: children.to_vec().into(),
        },
    )
    .ok()
    .unwrap();
    for &child in children {
        ec.add_component(child, ChildComponent { parent }).ok().unwrap();
    }
}

fn follow_cursor(
    input: Res<InputResource>,
    time: Res<TimeResource>,
    mut cursors: WorldQuery<(&mut Cursor, &mut LocalRectComponent)>,
) {
    let target = input.mouse.position.map(|x| x as f32);

    for (cursor, rect) in cursors.iter_mut() {
        let current = rect.offset.map(|x| x.val);
        let next = Vec2::from_fn(|i| {
            damp(
                current[i],
                target[i],
                &mut cursor.velocity[i],
                CURSOR_SMOOTH_TIME,
                CURSOR_DAMPING,
                time.delta_time,
            )
        });
        rect.offset = next.map(UiVal::px);
    }
}
