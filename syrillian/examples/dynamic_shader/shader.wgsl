fn tri(x_og: f32, y_og: f32) -> bool {
  let x = x_og * (cos(system.time) + 2) * 0.75;
  let y = y_og * (cos(system.time) + 2) * 0.75;

  if x > 0.25 || x < -0.25 || y > 0.25 || y < -0.25 {
    return false;
  }

  return -y - abs(x) * 2 > -0.25;
}

@fragment
fn fs_main(in: FInput) -> @location(0) vec4<f32> {
  let time = system.time;
  let slices = 20.;
  let uv_x = in.uv.x;
  let uv_y = in.uv.y;
  let i_x = round(uv_x * slices) / slices;
  let i_y = round(uv_y * slices) / slices;
  var inner_x = (uv_x % (1. / slices)) * slices;
  var inner_y = (uv_y % (1. / slices)) * slices;
  if inner_x > 0.5 {
    inner_x = (1 - inner_x) * 2;
  }
  if inner_y > 0.5 {
    inner_y = (1 - inner_y) * 2;
  }

  let tri_thing = 1 - f32(tri((uv_x - 0.5), (uv_y - 0.5)));

  let amp_s = sin(time + f32(i_x * 2)) / 2;
  let amp_c = cos(time + f32(i_x * 2) + 1);
  let opacity_x = round(inner_x - amp_s);
  let opacity_y = round(inner_y - amp_c);

  let opacity = (opacity_x * opacity_y);

  var color = vec4(1.0, in.uv, 1.0) * opacity * tri_thing;
  color.a = 1.0;

  return color;
}
