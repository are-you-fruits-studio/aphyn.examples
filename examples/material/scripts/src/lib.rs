//! Custom shaders and materials: ten small cubes in two rows spin at different speeds, half of them
//! a steady white and half blinking between orange and cyan once a second. A vignette darkens the
//! edges of the window.
//!
//! The cubes use a WGSL shader written for this example (`unlit_color.wgsl`), composed with the
//! engine's WGSL libraries and wrapped in a `MaterialTemplate` whose `color` input is bound to the
//! shader's uniform. Two materials are created from that template, one with the default color and
//! one with its own; the blinking is done by changing the second material's data at runtime. The
//! vignette (`vignette.wgsl`) is a custom `RenderFeature` with a post-processing pass that copies
//! the rendered frame and draws it back through a fullscreen triangle with raw GPU calls.
//!
//! Engine modules: `aphyn_render` (shader, material template, material, mesh, `WgslComposer`),
//! `aphyn_render_features` (`RenderFeature`, render pass orders, mesh, material and camera
//! components), `aphyn_render_core` (GPU shader, texture, sampler and draw call),
//! `aphyn_asset_storage`, `aphyn_transform`, `aphyn_ecs`, `aphyn_app` (`TimeResource`).

use aphyn::*;

const ORANGE: Vec4<f32> = Vec4::new(1.0, 0.45, 0.1, 1.0);
const CYAN: Vec4<f32> = Vec4::new(0.1, 0.8, 0.9, 1.0);

aphyn_entry_point!(init_app);

pub fn init_app(ctx: AppInitCtx) {
    let AppInitCtx {
        mut data,
        mut behavior,
        render,
        ..
    } = ctx;

    assert!(render.add_feature("vignette", VignetteFeature::default()));

    let mut start = behavior.as_mut();
    let mut start = start.get_mut(Schedule::Start);
    start.insert_system(spawn_cubes);
    start.order_group(SYSTEM_GROUP_RENDER).before_system(spawn_cubes);

    let mut update = behavior.as_mut();
    let mut update = update.get_mut(Schedule::Update);
    update.insert_system(rotate_cubes);
    update.insert_system(blink_material);

    spawn_camera(data.as_mut());
}

#[derive(Resource)]
struct ExampleMaterialsResource {
    blinking: AssetHandle<Material>,
}

#[derive(Default)]
struct BlinkState {
    last_second: u64,
}

#[derive(Component)]
struct SpinningComponent {
    speed: f32,
}

fn spawn_camera(data: WorldDataMut) {
    let mut ec = data.into_entities_mut();

    let camera = ec.create_entity();
    ec.set_components(
        camera,
        (
            LocalTransform {
                position: Vec3::new(0.0, 0.0, -4.0),
                ..LocalTransform::IDENTITY
            },
            CameraComponent(Camera {
                near: 0.1,
                far: 100.0,
                projection: CameraProjection::Perspective {
                    fov: 70_f32.to_radians(),
                },
            }),
        ),
    );
}

// One pass that writes the `color` input into the `color` member of the uniform at group 1, binding 0.
fn unlit_color_template(shader: AssetHandle<Shader>) -> MaterialTemplate {
    let mut template = MaterialTemplate::default();
    template.inputs.insert(
        "color".into(),
        MaterialInput::new(MaterialDataValue::Vec4(Vec4::new(1.0, 1.0, 1.0, 1.0))),
    );
    template.passes.push(MaterialTemplatePass {
        shader,
        order: RENDER_ORDER_OPAQUE,
        target: MaterialPassTarget::Main,
        values: vec![MaterialValueBinding {
            input: "color".into(),
            group: 1,
            binding: 0,
            member: "color".into(),
        }]
        .into(),
        textures: FfiVec::new(),
    });

    template
}

fn spawn_cubes(mut world: WorldDataMut) {
    let Some(render_api) = world
        .as_ref()
        .resources()
        .get::<RenderApiResource>()
        .map(RenderApiResource::clone)
    else {
        return;
    };

    let corners = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [0.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
    ];
    let vertices = corners.map(|corner: [f32; 3]| StandardVertex {
        position: corner.map(|c| (c * 2.0 - 1.0) * 0.35),
        ..Default::default()
    });
    let indices = [
        0, 1, 3, 0, 3, 2, 0, 4, 5, 0, 5, 1, 0, 6, 4, 0, 2, 6, 1, 7, 3, 1, 5, 7, 2, 7, 6, 2, 3, 7, 4, 6, 7, 4, 7, 5,
    ];
    let mesh = render_api.create_mesh(&vertices, &indices, None);

    let code = WgslComposer::with_engine_libraries()
        .compose("unlit_color.wgsl", include_str!("unlit_color.wgsl"), &[])
        .unwrap();
    let shader = render_api
        .create_shader("unlit_color.wgsl", &code, GpuShaderDesc::default(), None)
        .unwrap();

    let mut world_mut = world.as_mut();
    let mut resources = world_mut.resources_mut();

    let shader = resources.get_mut::<AssetStorageResource<Shader>>().unwrap().insert(shader);
    let template = resources
        .get_mut::<AssetStorageResource<MaterialTemplate>>()
        .unwrap()
        .insert(unlit_color_template(shader));
    let mesh = resources
        .get_mut::<AssetStorageResource<StandardMesh>>()
        .unwrap()
        .insert(mesh);

    let (steady, blinking) = {
        let templates = resources.get::<AssetStorageResource<MaterialTemplate>>().unwrap();
        let shaders = resources.get::<AssetStorageResource<Shader>>().unwrap();

        let steady = render_api.create_material(templates, shaders, template.clone(), MaterialData::new());

        let mut blinking_data = MaterialData::new();
        blinking_data.insert("color", MaterialDataValue::Vec4(ORANGE));
        let blinking = render_api.create_material(templates, shaders, template, blinking_data);

        (steady, blinking)
    };

    let materials = resources.get_mut::<AssetStorageResource<Material>>().unwrap();
    let (steady, blinking) = (materials.insert(steady), materials.insert(blinking));
    resources.insert(ExampleMaterialsResource {
        blinking: blinking.clone(),
    });

    let mut ec = world.entities_mut();

    for i in 0..10_u32 {
        let column = (i % 5) as f32;
        let row = (i / 5) as f32;
        let material = if i.is_multiple_of(2) {
            steady.clone()
        } else {
            blinking.clone()
        };

        let entity = ec.create_entity();
        ec.set_components(
            entity,
            (
                LocalTransform {
                    position: Vec3::new((column - 2.0) * 1.2, (row - 0.5) * 1.4, 0.0),
                    ..LocalTransform::IDENTITY
                },
                MeshComponent { mesh: mesh.clone() },
                MaterialComponent { material },
                SpinningComponent {
                    speed: 0.5 + i as f32 * 0.15,
                },
            ),
        );
    }
}

fn rotate_cubes(time: Res<TimeResource>, mut query: WorldQuery<(&mut LocalTransform, &SpinningComponent)>) {
    for (transform, spinning) in query.iter_mut() {
        let angle = time.frame_time * spinning.speed as f64;
        transform.rotation = Quat::rotation_y(angle) * Quat::rotation_x(angle * 0.7);
    }
}

// Changing a material's data recolors every entity that uses it.
fn blink_material(
    time: Res<TimeResource>,
    mut state: Local<BlinkState>,
    example_materials: Option<Res<ExampleMaterialsResource>>,
    mut materials: ResMut<AssetStorageResource<Material>>,
) {
    let Some(example_materials) = example_materials else {
        return;
    };
    let second = time.frame_time as u64;
    if second == state.last_second {
        return;
    }
    state.last_second = second;

    let Some(material) = materials.get_mut(&example_materials.blinking) else {
        return;
    };
    let Some(color) = material.data.get_vec4_mut("color") else {
        return;
    };

    *color = if second.is_multiple_of(2) { ORANGE } else { CYAN };
}

struct VignetteGpu {
    shader: GpuShader,
    sampler: GpuSampler,
    source: GpuTexture,
}

#[derive(Default)]
struct VignetteFeature {
    gpu: Option<VignetteGpu>,
}

impl RenderFeature for VignetteFeature {
    fn add_passes(&mut self, _world: WorldDataMut, passes: &mut RenderPassWriter<Self>) {
        passes.add(RENDER_ORDER_POST_PROCESSING, |this, ctx| this.draw(ctx));
    }
}

impl VignetteFeature {
    fn draw(&mut self, ctx: &mut RenderPassCtx) {
        let core = ctx.render_api.core();
        let Some(current) = ctx.world.resources().get::<CurrentRenderingResource>() else {
            return;
        };
        let target = current.target.texture();

        // GPU objects are created on the first frame, when the target's size and format are known
        if self.gpu.is_none() {
            let desc = GpuShaderDesc {
                cull_mode: GpuCullMode::None,
                depth_write: false,
                depth_compare: GpuCompareFunction::Always,
                ..Default::default()
            };
            let Some(shader) = create_shader(core, "vignette.wgsl", include_str!("vignette.wgsl"), desc) else {
                return;
            };

            self.gpu = Some(VignetteGpu {
                shader,
                sampler: core.create_sampler(Some("vignette"), GpuFilterMode::Linear),
                source: core.create_texture(Some("vignette source"), target.size(), target.format(), 1),
            });
        }
        let gpu = self.gpu.as_mut().unwrap();

        if gpu.source.size() != target.size() || gpu.source.format() != target.format() {
            gpu.source = core.create_texture(Some("vignette source"), target.size(), target.format(), 1);
        }

        // a pass cannot sample the texture it draws into, so it reads from a copy
        if !core.copy_texture(target, &gpu.source) {
            return;
        }

        core.draw(
            &GpuRenderPass {
                shader: &gpu.shader,
                target: Some(GpuRenderPassTarget {
                    view: target,
                    mip_level: 0,
                    ops: GpuPassOps::Load,
                }),
                depth: None,
                bind_groups: &[&[
                    (
                        0,
                        GpuBinding::Texture {
                            texture: &gpu.source,
                            mip_level: None,
                        },
                    ),
                    (1, GpuBinding::Sampler(&gpu.sampler)),
                ]],
                vertex_buffers: &[],
                viewport: None,
                scissor: None,
            },
            // one triangle that covers the whole target
            0..3,
            0..1,
        );
    }
}

fn create_shader(core: &RenderApi, name: &str, source: &str, desc: GpuShaderDesc) -> Option<GpuShader> {
    let code = match WgslComposer::with_engine_libraries().compose(name, source, &[]) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            return None;
        }
    };

    core.create_shader(Some(name), &code, desc)
}
