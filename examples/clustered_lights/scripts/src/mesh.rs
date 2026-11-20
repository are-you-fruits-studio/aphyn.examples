use std::f32::consts::PI;

use aphyn::*;

// A unit cube with flat outward normals. The triangles are clockwise seen from outside: that is the
// front face for the engine's projection.
pub fn make_cube() -> (Vec<StandardVertex>, Vec<u32>) {
    let h = 0.5_f32;
    let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
        ([0.0, 0.0, 1.0], [[-h, -h, h], [h, -h, h], [h, h, h], [-h, h, h]]),
        ([0.0, 0.0, -1.0], [[h, -h, -h], [-h, -h, -h], [-h, h, -h], [h, h, -h]]),
        ([1.0, 0.0, 0.0], [[h, -h, h], [h, -h, -h], [h, h, -h], [h, h, h]]),
        ([-1.0, 0.0, 0.0], [[-h, -h, -h], [-h, -h, h], [-h, h, h], [-h, h, -h]]),
        ([0.0, 1.0, 0.0], [[-h, h, h], [h, h, h], [h, h, -h], [-h, h, -h]]),
        ([0.0, -1.0, 0.0], [[-h, -h, -h], [h, -h, -h], [h, -h, h], [-h, -h, h]]),
    ];
    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for (normal, corners) in faces {
        let [tx, ty, tz] = [0, 1, 2].map(|i| corners[1][i] - corners[0][i]);
        let base = vertices.len() as u32;
        for (position, uv) in corners.into_iter().zip(uvs) {
            vertices.push(StandardVertex {
                position,
                normal,
                tangent: [tx, ty, tz, 1.0],
                color: [1.0; 4],
                uv,
            });
        }
        indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
    }
    (vertices, indices)
}

// A UV sphere of diameter 1, its tangent along the longitude so it is never parallel to the normal.
pub fn make_sphere(segments: u32, rings: u32) -> (Vec<StandardVertex>, Vec<u32>) {
    let mut vertices = Vec::with_capacity(((segments + 1) * (rings + 1)) as usize);
    let mut indices = Vec::with_capacity((segments * rings * 6) as usize);

    for ring in 0..=rings {
        let v = ring as f32 / rings as f32;
        let theta = v * PI;
        for segment in 0..=segments {
            let u = segment as f32 / segments as f32;
            let phi = u * 2.0 * PI;
            let normal = [theta.sin() * phi.cos(), theta.cos(), theta.sin() * phi.sin()];
            vertices.push(StandardVertex {
                position: normal.map(|x| x * 0.5),
                normal,
                tangent: [-phi.sin(), 0.0, phi.cos(), 1.0],
                color: [1.0; 4],
                uv: [u, v],
            });
        }
    }

    let row = segments + 1;
    for ring in 0..rings {
        for segment in 0..segments {
            let a = ring * row + segment;
            let b = a + row;
            indices.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    (vertices, indices)
}

pub fn upload(world: &mut WorldDataMut, (vertices, indices): (Vec<StandardVertex>, Vec<u32>)) -> AssetHandle<StandardMesh> {
    let mesh = world
        .resources()
        .get::<RenderApiResource>()
        .unwrap()
        .create_mesh(&vertices, &indices, None);
    world
        .resources_mut()
        .get_mut::<AssetStorageResource<StandardMesh>>()
        .unwrap()
        .insert(mesh)
}
