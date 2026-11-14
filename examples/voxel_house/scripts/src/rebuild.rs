use std::fmt::Write;

use aphyn::*;

use crate::house::spawn_house;
use crate::materials::Materials;
use crate::physics::Sim;

pub const DEFAULT_LEVEL: i32 = 2;

// the mesh and materials, kept so the house can be spawned again at runtime
#[derive(Resource, Clone)]
pub struct HouseAssets {
    pub cube: AssetHandle<StandardMesh>,
    pub materials: Materials,
}

#[derive(Resource, Default)]
pub struct Cubes {
    pub level: i32,
    pub ids: Vec<EntityId>,
}

#[derive(Component)]
pub struct CountText;

#[derive(Default)]
pub struct ShownCount(Option<usize>);

const LEVEL_KEYS: [(KeyCode, KeyCode); 9] = [
    (KeyCode::Digit1, KeyCode::Numpad1),
    (KeyCode::Digit2, KeyCode::Numpad2),
    (KeyCode::Digit3, KeyCode::Numpad3),
    (KeyCode::Digit4, KeyCode::Numpad4),
    (KeyCode::Digit5, KeyCode::Numpad5),
    (KeyCode::Digit6, KeyCode::Numpad6),
    (KeyCode::Digit7, KeyCode::Numpad7),
    (KeyCode::Digit8, KeyCode::Numpad8),
    (KeyCode::Digit9, KeyCode::Numpad9),
];

// Keys 1-9: destroy every cube and spawn the house again with N^3 cubes per voxel.
pub fn rebuild_house(mut world: WorldDataMut) {
    let level = {
        let resources = world.resources();
        let keyboard = &resources.get::<InputResource>().unwrap().keyboard;
        LEVEL_KEYS
            .iter()
            .position(|(digit, numpad)| keyboard.is_just_pressed(*digit) || keyboard.is_just_pressed(*numpad))
            .map(|i| i as i32 + 1)
    };
    let Some(level) = level else { return };
    if level == world.resources().get::<Cubes>().unwrap().level {
        return;
    }

    let old = std::mem::take(&mut world.resources_mut().get_mut::<Cubes>().unwrap().ids);
    let mut ec = world.entities_mut();
    for entity in old {
        ec.destroy_entity(entity);
    }

    let assets = world.resources().get::<HouseAssets>().unwrap().clone();
    let ids = spawn_house(&mut world, &assets.cube, &assets.materials, level);

    let mut resources = world.resources_mut();
    *resources.get_mut::<Cubes>().unwrap() = Cubes { level, ids };
    resources.get_mut::<Sim>().unwrap().exploded = false;
}

pub fn write_cube_count(
    cubes: Res<Cubes>,
    mut shown: Local<ShownCount>,
    mut texts: WorldQuery<&mut TextComponent, WithFilter<CountText>>,
) {
    let count = cubes.ids.len();
    if shown.0 == Some(count) {
        return;
    }
    shown.0 = Some(count);
    for text in texts.iter_mut() {
        text.text.clear();
        write!(text.text, "cubes: {count}  (level {}, keys 1-9)", cubes.level).unwrap();
    }
}
