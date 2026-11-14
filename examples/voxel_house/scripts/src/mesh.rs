use aphyn::*;

// A unit cube with flat outward normals and white vertex colors, so the material's color tints it.
// The two triangles of every face are listed in reverse (clockwise seen from outside): that is the
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
        // along the face's u axis; a tangent parallel to the normal turns the lit shader's normal into NaN
        let [tx, ty, tz] = [0, 1, 2].map(|i| corners[1][i] - corners[0][i]);
        let base = vertices.len() as u32;
        for (position, uv) in corners.into_iter().zip(uvs) {
            vertices.push(StandardVertex {
                position,
                normal,
                tangent: [tx, ty, tz, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
                uv,
            });
        }
        indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
    }
    (vertices, indices)
}

pub fn upload_cube(world: &mut WorldDataMut) -> AssetHandle<StandardMesh> {
    let (vertices, indices) = make_cube();
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
