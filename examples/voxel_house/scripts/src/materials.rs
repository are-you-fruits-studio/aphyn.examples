use aphyn::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Floor,
    Wall,
    Roof,
    Door,
    Glass,
    Brick,
    Trunk,
    Leaves,
}

const KINDS: [(Kind, [f32; 3]); 8] = [
    (Kind::Floor, [0.45, 0.33, 0.22]),
    (Kind::Wall, [0.93, 0.86, 0.70]),
    (Kind::Roof, [0.75, 0.22, 0.18]),
    (Kind::Door, [0.40, 0.24, 0.12]),
    (Kind::Glass, [0.45, 0.75, 0.95]),
    (Kind::Brick, [0.55, 0.30, 0.25]),
    (Kind::Trunk, [0.38, 0.26, 0.15]),
    (Kind::Leaves, [0.22, 0.60, 0.25]),
];
// three slightly different shades per kind, so the walls do not look like flat plastic
const SHADES: [f32; 3] = [0.92, 1.0, 1.08];

#[derive(Resource, Clone)]
pub struct Materials {
    voxels: Vec<[AssetHandle<Material>; 3]>,
    pub ground: AssetHandle<Material>,
    pub hud_text: AssetHandle<Material>,
}

impl Materials {
    pub fn create(world: &mut WorldDataMut) -> Self {
        let voxels = KINDS
            .iter()
            .map(|(_, c)| SHADES.map(|s| lit(world, Vec4::new(c[0] * s, c[1] * s, c[2] * s, 1.0))))
            .collect();
        let ground = lit(world, Vec4::new(0.30, 0.50, 0.24, 1.0));

        let atlas = world
            .resources()
            .get::<StandardUiAssetsResource>()
            .unwrap()
            .texture_text_px_8_12
            .clone();
        let mut text_data = MaterialData::new();
        text_data.insert("color_tex", MaterialDataValue::Texture(atlas));
        let hud_text = insert(world, BUILTIN_TEMPLATE_UI_OPAQUE_UNLIT, text_data);

        Self {
            voxels,
            ground,
            hud_text,
        }
    }

    pub fn voxel(&self, kind: Kind, shade: usize) -> &AssetHandle<Material> {
        let i = KINDS.iter().position(|(k, _)| *k == kind).unwrap();
        &self.voxels[i][shade % 3]
    }
}

fn lit(world: &mut WorldDataMut, color: Vec4<f32>) -> AssetHandle<Material> {
    let mut data = MaterialData::new();
    data.insert("color", MaterialDataValue::Vec4(color));
    data.insert("metallic", MaterialDataValue::F32(0.0));
    data.insert("roughness", MaterialDataValue::F32(0.85));
    insert(world, BUILTIN_TEMPLATE_OPAQUE_LIT, data)
}

fn insert(world: &mut WorldDataMut, template: &str, data: MaterialData) -> AssetHandle<Material> {
    let material = create_material_from_template_key(world.resources(), template, data).unwrap();
    world
        .resources_mut()
        .get_mut::<AssetStorageResource<Material>>()
        .unwrap()
        .insert(material)
}
