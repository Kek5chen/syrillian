const PI: f32 = 3.14159265359;
const AMBIENT_STRENGTH: f32 = 0.1;

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

fn spot_light(in: FInput, light: Light, world_normal: vec3<f32>, view_dir: vec3<f32>, base_color: vec3<f32>) -> vec3<f32> {
    // Vector from fragment to light
    var L = light.position - in.position;
    let dist = length(L);
    L = L / dist;

    // Lambert
    let NdotL = max(dot(world_normal, L), 0.0);
    if NdotL <= 0.0 { return vec3<f32>(0.0); }

    // Spotlight cone factor using cos angles to avoid acos
    let inner = min(light.inner_angle, light.outer_angle);
    let outer = max(light.inner_angle, light.outer_angle);
    let cosInner = cos(inner);
    let cosOuter = cos(outer);

    // dir_to_frag is from light to fragment
    let dir_to_frag = normalize(in.position - light.position);
    let cosTheta = dot(normalize(light.direction), dir_to_frag);

    // Smooth penumbra from outer (0) to inner (1)
    let spot = smoothstep(cosOuter, cosInner, cosTheta);

    // Range falloff with a soft edge near the limit
    let range_fade = 1.0 - smoothstep(light.range * 0.85, light.range, dist);
    let attenuation = calculate_attenuation(dist, light.range) * range_fade * spot;

    let radiance = light.color * light.intensity * attenuation;

    // Diffuse
    var lit_color = base_color * NdotL * radiance;

    // Specular
    let spec = calculate_specular(L, view_dir, world_normal, material.shininess);
    lit_color = lit_color + spec * radiance;

    if (light.shadow_map_id != 0xffffffffu) {
        let uvz = spot_shadow_uvz(light, in.position);
        if (all(uvz >= vec3<f32>(0.0)) && all(uvz <= vec3<f32>(1.0))) {
            let slope = 1.0 - max(dot(world_normal, L), 0.0);
            let bias  = 0.0000 + 0.0001 * slope;
            let layer = f32(light.shadow_map_id);
            let vis   = pcf_3x3(shadow_maps, shadow_sampler, vec4<f32>(uvz.xy, uvz.z - bias, layer));
            lit_color *= vis;
        }
    }

    return lit_color;
}

fn point_light(in: FInput, light: Light, world_normal: vec3<f32>, view_dir: vec3<f32>, base_color: vec3<f32>) -> vec3<f32> {
    // Vector from fragment to light
    var L = light.position - in.position;
    let dist = length(L);
    L = L / max(dist, 1e-6);

    // Lambert
    let NdotL = max(dot(world_normal, L), 0.0);
    if NdotL <= 0.0 { return vec3<f32>(0.0); }

    let range_fade = 1.0 - smoothstep(light.range * 0.85, light.range, dist);
    let attenuation = calculate_attenuation(dist, light.range) * range_fade;
    let radiance = light.color * light.intensity * attenuation;

    // Diffuse
    var lit_color = base_color * NdotL * radiance;

    let spec = calculate_specular(L, view_dir, world_normal, material.shininess);
    lit_color = lit_color + spec * radiance;

    return lit_color;
}

fn sun_light(in: FInput, light: Light, world_normal: vec3<f32>, view_dir: vec3<f32>, base_color: vec3<f32>) -> vec3<f32> {
    let light_dot = clamp(dot(world_normal, light.direction), 0.0, 1.0);
    let color = base_color * light_dot * light.intensity;

    return color;
}

@fragment
fn fs_main(in: FInput) -> @location(0) vec4<f32> {
    // Color
    var base_color: vec4<f32>;
    if material.use_diffuse_texture != 0u {
        base_color = textureSample(t_diffuse, s_diffuse, in.uv);
    } else {
        base_color = vec4<f32>(material.diffuse, 1.0);
    }

    // Discard Alpha
    if base_color.a < 0.1 {
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

    let count = light_count;
    for (var i: u32 = 0u; i < count; i = i + 1u) {
        let light = lights[i];

        if light.type_id == LIGHT_TYPE_POINT {
            lit_color = lit_color + point_light(in, light, world_normal, view_dir, base_color.xyz);
        } else if light.type_id == LIGHT_TYPE_SUN {
            lit_color = lit_color + sun_light(in, light, world_normal, view_dir, base_color.xyz);
        } else if light.type_id == LIGHT_TYPE_SPOT {
            lit_color = lit_color + spot_light(in, light, world_normal, view_dir, base_color.xyz);
        }
    }

    let final_color = vec4(lit_color, base_color.a * material.opacity);

    return final_color;
}



// Shadow stuff
fn stable_up(dir: vec3<f32>) -> vec3<f32> {
    let y = vec3<f32>(0.0, 1.0, 0.0);
    let x = vec3<f32>(1.0, 0.0, 0.0);
    return select(y, x, abs(dot(normalize(dir), y)) > 0.99);
}

fn view_look_at_rh(pos: vec3<f32>, target_pos: vec3<f32>, up: vec3<f32>) -> mat4x4<f32> {
    let f = normalize(target_pos - pos);
    let s = normalize(cross(f, up));
    let u = cross(s, f);

    return mat4x4<f32>(
        vec4<f32>(  s.x,   u.x,  -f.x, 0.0),
        vec4<f32>(  s.y,   u.y,  -f.y, 0.0),
        vec4<f32>(  s.z,   u.z,  -f.z, 0.0),
        vec4<f32>(-dot(s, pos),
                  -dot(u, pos),
                   dot(f, pos), 1.0)
    );
}

fn proj_perspective(fovy: f32, near: f32, far: f32) -> mat4x4<f32> {
    let fct = 1.0 / tan(fovy / 2.0);
    return mat4x4<f32>(
        vec4<f32>( fct, 0.0, 0.0, 0.0),
        vec4<f32>( 0.0, fct, 0.0, 0.0),
        vec4<f32>( 0.0, 0.0, (far + near) / (near - far), -1.0),
        vec4<f32>( 0.0, 0.0, far * near * 2.0 / (near - far), 0.0)
    );
}

fn spot_shadow_uvz(light: Light, world_pos: vec3<f32>) -> vec3<f32> {
    let up   = stable_up(light.direction);
    let view = view_look_at_rh(light.position, light.position + light.direction, up);

    let fovy = max(0.001, 2.0 * max(light.inner_angle, light.outer_angle));
    let near = 0.05;
    let far  = max(near + 0.01, light.range);
    let proj = proj_perspective(fovy, near, far);

    let clip = proj * view * vec4<f32>(world_pos, 1.0);
    let ndc  = clip.xyz / max(1e-6, clip.w);

    var uv = ndc.xy * 0.5 + 0.5;
    uv.y = 1.0 - uv.y;

    return vec3<f32>(uv, ndc.z);
}

fn pcf_3x3(depthTex: texture_depth_2d_array,
           cmpSampler: sampler_comparison,
           uvzLayer: vec4<f32>) -> f32
{
    let layer = u32(uvzLayer.w + 0.5);
    let dims  = vec2<f32>(textureDimensions(depthTex, 0));
    let texel = 1.0 / dims;

    var sum = 0.0;
    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            let ofs = vec2<f32>(f32(dx), f32(dy)) * texel;
            sum += textureSampleCompare(depthTex, cmpSampler, uvzLayer.xy + ofs, layer, uvzLayer.z);
        }
    }
    return sum / 9.0;
}