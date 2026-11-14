use std::collections::HashMap;

use aphyn::*;

use crate::materials::{Kind, Materials};
use crate::physics::Voxel;

// edge length of one voxel in world units
pub const VOXEL: f32 = 0.5;

const HOUSE_WIDTH_X: i32 = 9;
const HOUSE_DEPTH_Z: i32 = 7;
const WALL_H: i32 = 4;

type Cells = HashMap<(i32, i32, i32), Kind>;

// A sparse map of grid cells. Later writes replace earlier ones, so the door, windows and chimney
// simply overwrite wall and roof cells.
fn layout() -> Cells {
    let mut c = Cells::new();

    for x in 0..HOUSE_WIDTH_X {
        for z in 0..HOUSE_DEPTH_Z {
            c.insert((x, 0, z), Kind::Floor);
        }
    }
    for y in 1..=WALL_H {
        for x in 0..HOUSE_WIDTH_X {
            for z in 0..HOUSE_DEPTH_Z {
                if x == 0 || x == HOUSE_WIDTH_X - 1 || z == 0 || z == HOUSE_DEPTH_Z - 1 {
                    c.insert((x, y, z), Kind::Wall);
                }
            }
        }
    }
    // door at the front (z = 0) and windows
    for y in 1..=2 {
        c.insert((4, y, 0), Kind::Door);
    }
    for x in [1, 2, 6, 7] {
        c.insert((x, 3, 0), Kind::Glass);
        c.insert((x, 2, 0), Kind::Glass);
    }
    for z in [2, 4] {
        for y in [2, 3] {
            c.insert((0, y, z), Kind::Glass);
            c.insert((HOUSE_WIDTH_X - 1, y, z), Kind::Glass);
        }
    }
    for x in [3, 4, 5] {
        c.insert((x, 3, HOUSE_DEPTH_Z - 1), Kind::Glass);
    }
    // stepped gable roof, ridge along x, one voxel of overhang on every side
    let mut k = 0;
    while HOUSE_DEPTH_Z - 2 * k > 0 {
        let y = WALL_H + 1 + k;
        for x in -1..=HOUSE_WIDTH_X {
            for z in (k - 1)..=(HOUSE_DEPTH_Z - k) {
                c.insert((x, y, z), Kind::Roof);
            }
        }
        k += 1;
    }
    // chimney
    for y in (WALL_H + 1)..=(WALL_H + 5) {
        c.insert((6, y, 5), Kind::Brick);
    }
    // tree: a trunk and an ellipsoid of leaves
    for y in 1..=3 {
        c.insert((-4, y, 2), Kind::Trunk);
    }
    for x in -6..=-2 {
        for y in 4..=6 {
            for z in 0..=4 {
                let (dx, dy, dz) = (x + 4, y - 5, z - 2);
                if dx * dx + dy * dy * 2 + dz * dz <= 5 {
                    c.insert((x, y, z), Kind::Leaves);
                }
            }
        }
    }
    c
}

// deterministic per-cell hash that picks a shade
fn hash(x: i32, y: i32, z: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(73856093) ^ (y as u32).wrapping_mul(19349663) ^ (z as u32).wrapping_mul(83492791);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1e995);
    h ^ (h >> 15)
}

// Spawns the house with every layout cell split into `subdiv`^3 cubes and returns the entities.
pub fn spawn_house(world: &mut WorldDataMut, mesh: &AssetHandle<StandardMesh>, materials: &Materials, subdiv: i32) -> Vec<EntityId> {
    let cube_size = VOXEL / subdiv as f32;
    let mut cells: Vec<_> = layout().into_iter().collect();
    // HashMap order is random; sorting gives every cube the same seed, and so the same kick, on every run
    cells.sort_by_key(|((x, y, z), _)| (*y, *x, *z));

    let mut ec = world.entities_mut();
    let mut ids = Vec::with_capacity(cells.len() * (subdiv * subdiv * subdiv) as usize);
    for ((x, y, z), kind) in cells {
        let cell_min = Vec3::new(
            (x - HOUSE_WIDTH_X / 2) as f32 * VOXEL,
            y as f32 * VOXEL,
            (z - HOUSE_DEPTH_Z / 2) as f32 * VOXEL,
        );
        for sx in 0..subdiv {
            for sy in 0..subdiv {
                for sz in 0..subdiv {
                    let home = cell_min
                        + Vec3::new(
                            (sx as f32 + 0.5) * cube_size,
                            (sy as f32 + 0.5) * cube_size,
                            (sz as f32 + 0.5) * cube_size,
                        );
                    let shade = (hash(x * subdiv + sx, y * subdiv + sy, z * subdiv + sz) % 3) as usize;
                    let entity = ec.create_entity();
                    // GlobalTransform and ParentComponent up front avoid one archetype move per cube at up to 352k entities
                    ec.set_components(
                        entity,
                        (
                            GlobalTransform::IDENTITY,
                            ParentComponent::default(),
                            LocalTransform {
                                position: home,
                                rotation: Quat::IDENTITY,
                                scale: Vec3::splat(cube_size),
                            },
                            MeshComponent { mesh: mesh.clone() },
                            MaterialComponent {
                                material: materials.voxel(kind, shade).clone(),
                            },
                            Voxel::new(home, ids.len() as u32, cube_size),
                        ),
                    );
                    ids.push(entity);
                }
            }
        }
    }
    ids
}
