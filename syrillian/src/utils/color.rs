use nalgebra::Vector3;

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vector3<f32> {
    let h = h.rem_euclid(360.0);
    let c = v * s;
    let hp = h / 60.0;
    let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());
    let (r1, g1, b1) = match hp.floor() as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        5 => (c, 0.0, x),
        _ => (0.0, 0.0, 0.0),
    };
    Vector3::new(r1, g1, b1)
}
