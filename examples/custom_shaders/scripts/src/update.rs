use std::fmt::Write;

use aphyn::*;

#[derive(Component)]
pub struct Spin(pub f32);

#[derive(Component)]
pub struct OrbitingLight;

#[derive(Component)]
pub struct StatusText;

#[derive(Resource)]
pub struct DemoState {
    // materials drawn by the demo's own shaders, rebuilt after a reload
    pub custom_materials: Vec<AssetHandle<Material>>,
    reload_status: String,
}

impl Default for DemoState {
    fn default() -> Self {
        Self {
            custom_materials: Vec::new(),
            reload_status: "F5 reload shaders".to_string(),
        }
    }
}

// Materials whose template has a `time` input get the seconds since start.
pub fn write_time_into_materials(
    time: Res<TimeResource>,
    state: Res<DemoState>,
    mut materials: ResMut<AssetStorageResource<Material>>,
) {
    for handle in &state.custom_materials {
        let Some(material) = materials.get_mut(handle) else {
            continue;
        };

        if let Some(value) = material.data.get_f32_mut("time") {
            *value = time.frame_time as f32;
        }
    }
}

pub fn spin(time: Res<TimeResource>, mut query: WorldQuery<(&mut LocalTransform, &Spin)>) {
    for (transform, spin) in query.iter_mut() {
        transform.rotation = Quat::rotation_y(time.frame_time * spin.0 as f64);
    }
}

pub fn orbit_light(time: Res<TimeResource>, mut query: WorldQuery<&mut LocalTransform, WithFilter<OrbitingLight>>) {
    let (sin, cos) = (time.frame_time as f32 * 0.7).sin_cos();

    for transform in query.iter_mut() {
        transform.position = Vec3::new(cos * 7.0, sin * 3.5, -5.0);
    }
}

pub fn toggle_bloom(input: Res<InputResource>, mut bloom: ResMut<BloomResource>) {
    if input.keyboard.is_just_pressed(KeyCode::KeyB) {
        bloom.is_enabled = !bloom.is_enabled;
    }
}

// Loads every shader that came from an asset file again from its `.wgsl` file. A shader that fails
// to load keeps its previous version; the loader prints the error.
pub fn reload_shaders(
    input: Res<InputResource>,
    render_api: Res<RenderApiResource>,
    assets_dir: Res<AssetsDirResource>,
    templates: Res<AssetStorageResource<MaterialTemplate>>,
    mut shaders: ResMut<AssetStorageResource<Shader>>,
    mut materials: ResMut<AssetStorageResource<Material>>,
    mut state: ResMut<DemoState>,
) {
    if !input.keyboard.is_just_pressed(KeyCode::F5) {
        return;
    }

    // the engine's built-in shaders are created in code and have no asset metadata
    let keys = shaders
        .get_all()
        .filter(|(shader, _)| shader.meta().is_some())
        .filter_map(|(_, key)| key.map(str::to_string))
        .collect::<Vec<_>>();
    let loader = ShaderLoader { render_api: &render_api };
    let mut failed = Vec::new();

    for key in &keys {
        let Some(handle) = shaders.get_registered(key).cloned() else {
            continue;
        };
        let Some(meta) = shaders.get(&handle).and_then(Shader::meta).cloned() else {
            continue;
        };

        match loader.load_from_deserialized(meta, assets_dir.0.as_str()) {
            Some(shader) => *shaders.get_mut(&handle).unwrap() = shader,
            None => failed.push(key.as_str()),
        }
    }

    // a material keeps the GPU shader it was built with until it is rebuilt
    for handle in &state.custom_materials {
        if let Some(material) = materials.get_mut(handle) {
            render_api.rebuild_material_gpu(&templates, &shaders, material);
        }
    }

    let reloaded = keys.len() - failed.len();
    state.reload_status = if failed.is_empty() {
        format!("F5: {reloaded}/{} shaders reloaded", keys.len())
    } else {
        format!("F5: {reloaded}/{} shaders reloaded, failed {}", keys.len(), failed.join(" "))
    };
}

pub fn write_status(
    state: Res<DemoState>,
    bloom: Res<BloomResource>,
    mut query: WorldQuery<&mut TextComponent, WithFilter<StatusText>>,
) {
    let bloom = if bloom.is_enabled { "on" } else { "off" };

    for text in query.iter_mut() {
        text.text.clear();
        write!(text.text, "{} | B bloom: {bloom}", state.reload_status).unwrap();
    }
}
