// ----------------- Bones -------------------

fn sum4(v: vec4<f32>) -> f32 {
    return v.x + v.y + v.z + v.w;
}

fn skin_pos(p: vec4<f32>, idx: vec4<u32>, ow: vec4<f32>) -> vec4<f32> {
    if (length(ow) < 1e-6) {
        return p;
    }
    let w = normalize(ow);
    return (bones.mats[idx.x] * p) * w.x +
           (bones.mats[idx.y] * p) * w.y +
           (bones.mats[idx.z] * p) * w.z +
           (bones.mats[idx.w] * p) * w.w;
}

fn skin_dir(v: vec3<f32>, idx: vec4<u32>, w: vec4<f32>) -> vec3<f32> {
    if (length(w) < 1e-6) {
        return v;
    }

    let m0 = mat3x3<f32>(bones.mats[idx.x][0].xyz, bones.mats[idx.x][1].xyz, bones.mats[idx.x][2].xyz);
    let m1 = mat3x3<f32>(bones.mats[idx.y][0].xyz, bones.mats[idx.y][1].xyz, bones.mats[idx.y][2].xyz);
    let m2 = mat3x3<f32>(bones.mats[idx.z][0].xyz, bones.mats[idx.z][1].xyz, bones.mats[idx.z][2].xyz);
    let m3 = mat3x3<f32>(bones.mats[idx.w][0].xyz, bones.mats[idx.w][1].xyz, bones.mats[idx.w][2].xyz);

    let r =
        (m0 * v) * w.x +
        (m1 * v) * w.y +
        (m2 * v) * w.z +
        (m3 * v) * w.w;

    return normalize(r);
}

@vertex
fn vs_main(in: VInput) -> FInput {
    var out: FInput;

    let p_obj = vec4(in.position, 1.0);
    let n_obj = in.normal;
    let t_obj = in.tangent.xyz;

    let p_sk = skin_pos(p_obj, in.bone_idx, in.bone_w);
    let n_sk = skin_dir(n_obj, in.bone_idx, in.bone_w);
    let t_sk = skin_dir(t_obj, in.bone_idx, in.bone_w);

    let ws_pos = model.transform * p_sk;
    out.position = ws_pos.xyz;
    out.clip = camera.view_proj_mat * ws_pos;

    out.uv = in.uv;

    // FIXME: This is only correct for uniform scaling + rotation.
    // For non-uniform scaling, transform using the inverse transpose of the model matrix (normal_mat).
    // normal_mat needs to be passed into ModelData.
    out.normal = normalize((model.transform * vec4(n_sk, 0.0)).xyz);
    out.tangent = normalize((model.transform * vec4(t_sk, 0.0)).xyz);
    out.bitangent = cross(out.normal, out.tangent);

    out.bone_idx = in.bone_idx;
    out.bone_w = in.bone_w;

    return out;
}
