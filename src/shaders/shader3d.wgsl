const PI = 3.14159265359;
const AMBIENT_STRENGTH = 0.1;

@vertex
fn vs_main(in: VInput) -> VOutput {
    var out: VOutput;

    let world_pos_4 = model.model_mat * vec4<f32>(in.vpos, 1.0);
    out.world_pos = world_pos_4.xyz;
    out.clip_pos = camera.view_proj_mat * world_pos_4;

    out.tex_coords = vec2<f32>(in.vtex.x, 1.0 - in.vtex.y);

    // FIXME: This is only correct for uniform scaling + rotation.
    // For non-uniform scaling, transform using the inverse transpose of the model matrix (normal_mat).
    // normal_mat needs to be passed into ModelData.
    let normal_mat = mat3x3<f32>(model.model_mat[0].xyz, model.model_mat[1].xyz, model.model_mat[2].xyz); // Approximation if no normal_mat uniform
    out.world_normal = normalize((model.model_mat * vec4<f32>(in.vnorm, 0.0)).xyz);
    out.world_tangent = normalize((model.model_mat * vec4<f32>(in.vtan, 0.0)).xyz);

    // Recompute bitangent for guaranteed orthogonality.
    out.world_bitangent = cross(out.world_normal, out.world_tangent);

    out.bone_indices = in.vboneidx;
    out.bone_weights = in.vboneweights;

    return out;
}

fn calculate_attenuation(distance: f32, radius: f32) -> f32 {
    if radius <= 0.0 { return 1.0; }

    // Simple linear falloff
    //return clamp(1.0 - distance / radius, 0.0, 1.0);

    // Cubic Falloff is cooler
    let attenuation = 1.0 / (1.0 + 0.1 * distance + 0.01 * distance * distance);
    return clamp(attenuation, 0.0, 1.0);
}

fn calculate_specular(
    light_dir: vec3<f32>,
    view_dir: vec3<f32>,
    world_normal: vec3<f32>,
    light_color: vec3<f32>,
    shininess: f32
) -> vec3<f32> {
    let half_dir = normalize(light_dir + view_dir);
    let spec_angle = max(dot(world_normal, half_dir), 0.0);
    
    let spec_power = pow(spec_angle, max(shininess, 1.0));
    return light_color * spec_power;
}

fn get_normal_from_map(
    tex: texture_2d<f32>,
    samp: sampler,
    uv: vec2<f32>,
    world_norm: vec3<f32>,
    world_tan: vec3<f32>,
    world_bitan: vec3<f32>
) -> vec3<f32> {
    let tangent_normal = textureSample(tex, samp, uv).xyz;
    let unpacked_normal = normalize(tangent_normal * 2.0 - 1.0);

    let tbn = mat3x3<f32>(
        normalize(world_tan),
        normalize(world_bitan),
        normalize(world_norm)
    );

    return normalize(tbn * unpacked_normal);
}


@fragment
fn fs_main(in: VOutput) -> @location(0) vec4<f32> {
    // Color 
    var base_color: vec4<f32>;
    if material.use_diffuse_texture != 0u {
        base_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    } else {
        base_color = vec4<f32>(material.diffuse, 1.0);
    }

    // Discard Alpha 
    if base_color.a < 0.1 { // Example threshold
        discard;
    }

    // Normal 
    var world_normal: vec3<f32>;
    if material.use_normal_texture != 0u {
        world_normal = get_normal_from_map(
            t_normal, s_normal, in.tex_coords,
            in.world_normal, in.world_tangent, in.world_bitangent
        );
    } else {
        world_normal = normalize(in.world_normal);
    }

    // Lighting
    let view_dir = normalize(camera.pos - in.world_pos);

    var lit_color = base_color.rgb * AMBIENT_STRENGTH;

    let count = point_light_count;
    for (var i: u32 = 0u; i < count; i = i + 1u) {
        let light = point_lights[i];
        let light_dir = light.pos - in.world_pos;
        let distance = length(light_dir);

        if distance > light.radius {
            continue;
        }

        let light_dir_norm = normalize(light_dir);
        let attenuation = calculate_attenuation(distance, light.radius);
        let light_strength = light.color * light.intensity * attenuation;

        let diffuse_angle = max(dot(world_normal, light_dir_norm), 0.0);
        let diffuse_contrib = base_color.rgb * diffuse_angle * light_strength;
        lit_color = lit_color + diffuse_contrib;

        let specular_color = vec3(1.0, 1.0, 1.0);
        let specular_contrib = calculate_specular(
            light_dir_norm, view_dir, world_normal,
            specular_color * light_strength,
            material.shininess
        );
        lit_color = lit_color + specular_contrib;
    }

    let final_color = vec4(lit_color, base_color.a * material.opacity);

    return final_color;
}
