const PI = 3.14159265359;
const AMBIENT_STRENGTH = 0.1;

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
    shininess: f32
) -> f32 {
    let half_dir = normalize(light_dir + view_dir);
    let spec_angle = dot(world_normal, half_dir);
    
    let spec_power = pow(saturate(spec_angle), shininess);
    return spec_power;
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
        base_color = textureSample(t_diffuse, s_diffuse, in.uv);
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
            t_normal, s_normal, in.uv,
            in.normal, in.tangent, in.bitangent
        );
    } else {
        world_normal = normalize(in.normal);
    }

    // Lighting
    let view_dir = normalize(camera.position - in.position);

    var lit_color = base_color.rgb * AMBIENT_STRENGTH;

    let count = point_light_count;
    for (var i: u32 = 0u; i < count; i = i + 1u) {
        let light = point_lights[i];
        var light_dir = light.position - in.position;
        var distance = length(light_dir);
        light_dir = light_dir / distance;

        if distance > light.radius {
            continue;
        }

        let diffuse_angle = dot(world_normal, light_dir);

        if diffuse_angle <= 0 {
            continue;
        }

        let attenuation = calculate_attenuation(distance, light.radius);
        let light_strength = light.color * light.intensity * attenuation;

        let diffuse_contrib = base_color.rgb * diffuse_angle * light_strength;
        lit_color = lit_color + diffuse_contrib;

        distance = distance * distance;

        let specular_color = light.specular_color * light.specular_intensity;
        let specular_contrib = calculate_specular(
            light_dir, view_dir, world_normal,
            material.shininess
        );
        let specular = specular_contrib * specular_color / distance;
        lit_color = lit_color + specular;
    }

    let final_color = vec4(lit_color, base_color.a * material.opacity);

    return final_color;
}
