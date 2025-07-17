fn circle(ranged: f32) -> vec4<f32> {
  if ranged > 0.0 && ranged < 0.2 {
    return vec4(1.0);
  } else {
    return vec4(0.0);
  }
}

fn triangle(x_og: f32, y_og: f32) -> bool {
  let x = x_og * cos(system.time);
  let y = y_og * cos(system.time);

  if x > 0.25 || x < -0.25 || y > 0.25 || y < -0.25 {
    return false;
  }

  return -y - abs(x) * 2 > -0.25;
}

@fragment
fn fs_main(in: VOutput) -> @location(0) vec4<f32> {
  let x = sin(in.position.x);
  let y = cos(in.position.y);

  let dist = length(in.position);
  var color = circle(dist);

  let tx = sin(system.time) + y;
  let ty = cos(system.time) + x;
  let meow = (x * tx + y * ty) % 0.3;
  let meow2 = (x * ty + y * tx) % 0.3;

  if abs(meow) > 0.1 && abs(meow) < 0.2 {
    color += vec4(1.0, 0.0, 0.0, 0.0);
  }

  if triangle(in.position.x, in.position.y) {
    color = vec4(1.0, 0.0, 1.0, 0.0) - color;
  }

  return color;
}
