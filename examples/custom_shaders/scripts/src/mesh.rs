use std::f32::consts::PI;

use aphyn::*;

pub type MeshData = (Vec<StandardVertex>, Vec<u32>);

// A unit-radius sphere over the longitude range `theta_start..theta_end` (radians); `theta = 0`
// faces +Z and `theta = PI / 2` faces +X, so `0..PI` and `PI..2 * PI` are the right and left halves
// as seen by a camera looking along +Z.
pub fn make_sphere(theta_start: f32, theta_end: f32, columns: u32, rows: u32) -> MeshData {
    let mut vertices = Vec::with_capacity((columns as usize + 1) * (rows as usize + 1));

    for row in 0..=rows {
        let phi = PI * row as f32 / rows as f32;

        for column in 0..=columns {
            let theta = theta_start + (theta_end - theta_start) * column as f32 / columns as f32;
            let normal = [phi.sin() * theta.sin(), phi.cos(), phi.sin() * theta.cos()];

            vertices.push(StandardVertex {
                position: normal,
                normal,
                tangent: [theta.cos(), 0.0, -theta.sin(), 1.0],
                color: [1.0; 4],
                uv: [theta / (2.0 * PI), row as f32 / rows as f32],
            });
        }
    }

    (vertices, grid_indices(columns, rows))
}

// A unit plane in local XZ, centered on the origin, facing +Y.
pub fn make_plane(segments: u32) -> MeshData {
    let mut vertices = Vec::with_capacity((segments as usize + 1) * (segments as usize + 1));

    for row in 0..=segments {
        let z = row as f32 / segments as f32;

        for column in 0..=segments {
            let x = column as f32 / segments as f32;

            vertices.push(StandardVertex {
                position: [x - 0.5, 0.0, z - 0.5],
                normal: [0.0, 1.0, 0.0],
                tangent: [1.0, 0.0, 0.0, 1.0],
                color: [1.0; 4],
                uv: [x, z],
            });
        }
    }

    (vertices, grid_indices(segments, segments))
}

// Two triangles per grid cell, wound so that the outward side is the front face.
fn grid_indices(columns: u32, rows: u32) -> Vec<u32> {
    let stride = columns + 1;
    let mut indices = Vec::with_capacity(columns as usize * rows as usize * 6);

    for row in 0..rows {
        for column in 0..columns {
            let a = row * stride + column;
            let b = a + 1;
            let c = a + stride + 1;
            let d = a + stride;

            indices.extend_from_slice(&[a, b, c, a, c, d]);
        }
    }

    indices
}

pub fn upload(world: &mut WorldDataMut, (vertices, indices): MeshData) -> AssetHandle<StandardMesh> {
    let mesh = world
        .as_ref()
        .resources()
        .get::<RenderApiResource>()
        .unwrap()
        .create_mesh(&vertices, &indices, None);

    world
        .as_mut()
        .resources_mut()
        .get_mut::<AssetStorageResource<StandardMesh>>()
        .unwrap()
        .insert(mesh)
}
