fn lol(ranged: f32) -> vec4<f32> {
     if ranged > 1 {
        return vec4(0.0);
    } else {
        return vec4(1.0);
    }
}

@fragment
fn fs_main(in: VOutput) -> @location(0) vec4<f32> {
    let ranged = length(in.position) % 2;

    return lol(ranged);
}
