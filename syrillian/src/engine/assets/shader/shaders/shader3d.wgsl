const PI: f32 = 3.14159265359;
const AMBIENT_STRENGTH: f32 = 0.1;

fn saturate3(v: vec3<f32>) -> vec3<f32> { return clamp(v, vec3<f32>(0.0), vec3<f32>(1.0)); }
fn safe_rsqrt(x: f32) -> f32 { return inverseSqrt(max(x, 1e-8)); }
fn safe_normalize(v: vec3<f32>) -> vec3<f32> { return v * safe_rsqrt(dot(v, v)); }

// derive ggx roughness in [~0.045, 1] from blinn-phong shininess exponent.
fn shininess_to_roughness(n: f32) -> f32 {
    let a = sqrt(2.0 / (n + 2.0));
    return clamp(a, 0.045, 1.0);
}

// orthonormalize t against n to build a stable tbn basis
fn ortho_tangent(T: vec3<f32>, N: vec3<f32>) -> vec3<f32> {
    return safe_normalize(T - N * dot(N, T));
}

// fetch tangent-space normal and bring it to world space with a proper tbn
fn normal_from_map(
    tex: texture_2d<f32>, samp: sampler, uv: vec2<f32>,
    Nw: vec3<f32>, Tw: vec3<f32>, Bw: vec3<f32>
) -> vec3<f32> {
    let n_ts = textureSample(tex, samp, uv).xyz * 2.0 - 1.0; // [-1..1]
    let T = ortho_tangent(safe_normalize(Tw), safe_normalize(Nw));
    let B = safe_normalize(cross(Nw, T)) * sign(dot(Bw, cross(Nw, T))); // preserve handedness
    let TBN = mat3x3<f32>(T, B, safe_normalize(Nw));
    return safe_normalize(TBN * n_ts);
}

// ---------- Microfacet (GGX) BRDF ----------
fn D_ggx(NdotH: f32, a: f32) -> f32 {
    let a2 = a*a;
    let d = NdotH*NdotH*(a2 - 1.0) + 1.0;
    return a2 / (PI * d * d);
}

fn V_smith_ggx_correlated(NdotV: f32, NdotL: f32, a: f32) -> f32 {
    let a2 = a*a;
    let gv = NdotL * sqrt((NdotV * (1.0 - a2)) + a2);
    let gl = NdotV * sqrt((NdotL * (1.0 - a2)) + a2);
    return 0.5 / (gv + gl + 1e-7); // Vis = G/(4 NdotV NdotL)
}

fn fresnel_schlick(cosTheta: f32, F0: vec3<f32>) -> vec3<f32> {
    return F0 + (vec3<f32>(1.0) - F0) * pow(1.0 - cosTheta, 5.0);
}

// Simple energy split: kd = (1 - average(F)) for dielectrics (no metalness param yet)
fn diffuse_lambert(base: vec3<f32>) -> vec3<f32> {
    return base / PI;
}

// ACES Filmic tonemapping (linear → linear)
fn RRTAndODTFit(v: vec3<f32>) -> vec3<f32> {
    let a = v * (v + 0.0245786) - 0.000090537;
    let b = v * (0.983729 * v + 0.4329510) + 0.238081;
    return a / b;
}

// Filmic tonemap
fn tonemap_ACES(color: vec3<f32>) -> vec3<f32> {
    let ACES_IN = mat3x3<f32>(
        vec3<f32>(0.59719, 0.07600, 0.02840),
        vec3<f32>(0.35458, 0.90834, 0.13383),
        vec3<f32>(0.04823, 0.01566, 0.83777)
    );
    let ACES_OUT = mat3x3<f32>(
        vec3<f32>( 1.60475, -0.10208, -0.00327),
        vec3<f32>(-0.53108,  1.10813, -0.07276),
        vec3<f32>(-0.07367, -0.00605,  1.07602)
    );

    let v = ACES_IN * color;
    let r = RRTAndODTFit(v);
    let o = ACES_OUT * r;
    return clamp(o, vec3<f32>(0.0), vec3<f32>(1.0));
}

// Lottes "Neutral" tonemap (linear in -> linear out)
fn tonemap_neutral(x: vec3<f32>) -> vec3<f32> {
    let A = 0.22;
    let B = 0.30;
    let C = 0.10;
    let D = 0.20;
    let E = 0.01;
    let F = 0.30;
    let exposure = 1.0;
    let v = x * exposure;
    let y = ((v * (A * v + C * B) + D * E) / (v * (A * v + B) + D * F)) - (E / F);
    return clamp(y, vec3<f32>(0.0), vec3<f32>(1.0));
}
fn calculate_attenuation(distance: f32, radius: f32) -> f32 {
    if radius <= 0.0 { return 1.0; }
    let att = 1.0 / (1.0 + 0.09 * distance + 0.032 * distance * distance);
    return clamp(att, 0.0, 1.0);
}

fn shadow_visibility_spot(in_pos: vec3<f32>, N: vec3<f32>, L: vec3<f32>, light: Light) -> f32 {
    if (light.shadow_map_id == 0xffffffffu) { return 1.0; }

    let world_pos_bias = in_pos + N * 0.002;
    let uvz = spot_shadow_uvz(light, world_pos_bias);
    if !(all(uvz >= vec3<f32>(0.0)) && all(uvz <= vec3<f32>(1.0))) {
        return 1.0;
    }

    let slope = 1.0 - max(dot(N, L), 0.0);
    let bias  = 0.0001 * slope;
    let layer = f32(light.shadow_map_id);
    return pcf_3x3(shadow_maps, shadow_sampler, vec4<f32>(uvz.xy, uvz.z - bias, layer));
}

fn eval_spot(
    in_pos: vec3<f32>, N: vec3<f32>, V: vec3<f32>,
    base: vec3<f32>, F0: vec3<f32>, a: f32, light: Light
) -> vec3<f32> {
    var L = light.position - in_pos;
    let dist = length(L);
    L = L / max(dist, 1e-6);

    let NdotL = saturate(dot(N, L));
    if (NdotL <= 0.0) { return vec3<f32>(0.0); }

    // Smooth spot cone
    let inner = min(light.inner_angle, light.outer_angle);
    let outer = max(light.inner_angle, light.outer_angle);
    let cosInner = cos(inner);
    let cosOuter = cos(outer);
    let dir_to_frag = safe_normalize(in_pos - light.position);
    let cosTheta = dot(safe_normalize(light.direction), dir_to_frag);
    let spot = smoothstep(cosOuter, cosInner, cosTheta);

    // Distance falloff
    let range_fade  = 1.0 - smoothstep(light.range * 0.85, light.range, dist);
    let attenuation = calculate_attenuation(dist, light.range) * range_fade * spot;

    // Microfacet
    let NdotV = saturate(dot(N, V));
    let H     = safe_normalize(V + L);
    let NdotH = saturate(dot(N, H));
    let LdotH = saturate(dot(L, H));

    let D  = D_ggx(NdotH, a);
    let Vis= V_smith_ggx_correlated(NdotV, NdotL, a);
    let F  = fresnel_schlick(LdotH, F0);

    // Specular (Cook-Torrance)
    let spec = (F * (D * Vis)) * NdotL;

    // Diffuse energy conservation: kd = 1 - average(F) (no metalness yet, but gonna start going PBR soon)
    let ks = (F.x + F.y + F.z) / 3.0;
    let kd = (1.0 - ks);
    let diff = diffuse_lambert(base) * kd * NdotL;

    // Shadowing
    let vis = shadow_visibility_spot(in_pos, N, L, light);

    let radiance = light.color * (light.intensity * attenuation) * vis;
    return (diff + spec) * radiance;
}

fn eval_point(
    in_pos: vec3<f32>, N: vec3<f32>, V: vec3<f32>,
    base: vec3<f32>, F0: vec3<f32>, a: f32, light: Light
) -> vec3<f32> {
    var L = light.position - in_pos;
    let dist = length(L);
    L = L / max(dist, 1e-6);

    let NdotL = saturate(dot(N, L));
    if (NdotL <= 0.0) { return vec3<f32>(0.0); }

    let range_fade  = 1.0 - smoothstep(light.range * 0.85, light.range, dist);
    let attenuation = calculate_attenuation(dist, light.range) * range_fade;

    let NdotV = saturate(dot(N, V));
    let H     = safe_normalize(V + L);
    let NdotH = saturate(dot(N, H));
    let LdotH = saturate(dot(L, H));

    let D  = D_ggx(NdotH, a);
    let Vis= V_smith_ggx_correlated(NdotV, NdotL, a);
    let F  = fresnel_schlick(LdotH, F0);

    let ks = (F.x + F.y + F.z) / 3.0;
    let kd = (1.0 - ks);
    let diff = diffuse_lambert(base) * kd * NdotL;
    let spec = (F * (D * Vis)) * NdotL;

    let radiance = light.color * (light.intensity * attenuation);
    return (diff + spec) * radiance;
}

fn eval_sun(
    _in_pos: vec3<f32>, N: vec3<f32>, V: vec3<f32>,
    base: vec3<f32>, F0: vec3<f32>, a: f32, light: Light
) -> vec3<f32> {
    let L = safe_normalize(light.direction);
    let NdotL = saturate(dot(N, L));
    if (NdotL <= 0.0) { return vec3<f32>(0.0); }

    let NdotV = saturate(dot(N, V));
    let H     = safe_normalize(V + L);
    let NdotH = saturate(dot(N, H));
    let LdotH = saturate(dot(L, H));

    let D  = D_ggx(NdotH, a);
    let Vis= V_smith_ggx_correlated(NdotV, NdotL, a);
    let F  = fresnel_schlick(LdotH, F0);

    let ks = (F.x + F.y + F.z) / 3.0;
    let kd = (1.0 - ks);
    let diff = diffuse_lambert(base) * kd * NdotL;
    let spec = (F * (D * Vis)) * NdotL;

    let radiance = light.color * light.intensity;
    return (diff + spec) * radiance;
}

@fragment
fn fs_main(in: FInput) -> @location(0) vec4<f32> {
    // Base color (linear)
    var base_rgba: vec4<f32>;
    if material.use_diffuse_texture != 0u {
        base_rgba = textureSample(t_diffuse, s_diffuse, in.uv);
    } else {
        base_rgba = vec4<f32>(material.diffuse, 1.0);
    }

    // Alpha test
    if (base_rgba.a < 0.01) { discard; }

    // World normal
    var N: vec3<f32>;
    if material.use_normal_texture != 0u {
        N = normal_from_map(t_normal, s_normal, in.uv, in.normal, in.tangent, in.bitangent);
    } else {
        N = safe_normalize(in.normal);
    }

    let V = safe_normalize(camera.position - in.position);   // to viewer
    let base = saturate3(base_rgba.rgb);

    // convert blinn-phong shininess to ggx roughness (until pbr)
    let roughness = shininess_to_roughness(material.shininess);
    let a = roughness * roughness;

    // dielectric f0 (constant 4% reflectance). when adding metalness later, tint f0 by base.
    let F0 = vec3<f32>(0.04);

    // start with a dim ambient term (energy‑aware)
    var Lo = base * (AMBIENT_STRENGTH * (1.0 - 0.04)); // tiny spec energy loss

    // Lights
    let count = light_count;
    for (var i: u32 = 0u; i < count; i = i + 1u) {
        let Ld = lights[i];
        if (Ld.type_id == LIGHT_TYPE_POINT) {
            Lo += eval_point(in.position, N, V, base, F0, a, Ld);
        } else if (Ld.type_id == LIGHT_TYPE_SUN) {
            Lo += eval_sun(in.position, N, V, base, F0, a, Ld);
        } else if (Ld.type_id == LIGHT_TYPE_SPOT) {
            Lo += eval_spot(in.position, N, V, base, F0, a, Ld);
        }
    }


    // filmic tonemapping
    let color_tm = tonemap_ACES(Lo);

    // neutral tonemap
//    let color_tm = tonemap_neutral(Lo);

    // raw
    //let color_tm = Lo;

    // output linear. if swapchain is srgb, the hw will gamma‑encode.
    return vec4<f32>(color_tm, base_rgba.a * material.opacity);
}


// Shadow stuff
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
    let up   = light.up;
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