#use default_vertex
#use model

var<push_constant> color: vec4<f32>;

@vertex
fn vs_main(in: VInput) -> @builtin(position) vec4<f32> {
    let model_view_mat = camera.view_proj_mat * model.transform;

    var vpos = model_view_mat * vec4<f32>(in.position, 1.0);
    vpos.w += 0.0001; // lil w bump so it's not z fighting :>

    return vpos;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return color;
}

// this looks cool
//@fragment
//fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
//    return vec4(color.rgb, min(1.0, 3.0 + log(pos.w / 2)));
//}
