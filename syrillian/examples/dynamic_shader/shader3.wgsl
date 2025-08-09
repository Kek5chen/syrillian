// Create a 3D tunnel effect
@fragment
fn fs_main(in: VOutput) -> @location(0) vec4<f32> {
  let time = system.time;

  // Convert UV coordinates to centered coordinates (-0.5 to 0.5)
  let uv = in.uv - 0.5;

  // Calculate polar coordinates for tunnel effect
  let angle = atan2(uv.y, uv.x);
  let radius = length(uv);

  // Create tunnel depth based on radius
  let tunnel_depth = 1.0 / (radius + 0.1);

  // Animate the tunnel by moving through it
  let z_offset = time * 2.0;
  let tunnel_z = tunnel_depth + z_offset;

  // Create tunnel walls with animated patterns
  let wall_pattern = sin(angle * 8.0 + tunnel_z * 4.0) * 0.5 + 0.5;
  let depth_rings = sin(tunnel_z * 10.0) * 0.3 + 0.7;

  // Create a vignette effect to make it look more tunnel-like
  let vignette = 1.0 - smoothstep(0.0, 0.8, radius);

  // Color based on tunnel position and time
  let hue_shift = tunnel_z * 0.5 + time * 0.3;
  let r = sin(hue_shift) * 0.5 + 0.5;
  let g = sin(hue_shift + 2.094) * 0.5 + 0.5; // 2π/3 ≈ 2.094
  let b = sin(hue_shift + 4.188) * 0.5 + 0.5; // 4π/3 ≈ 4.188

  // Combine all effects
  let tunnel_intensity = wall_pattern * depth_rings * vignette;
  let final_color = vec3<f32>(r, g, b) * tunnel_intensity;

  // Add some glow effect at the center
  let center_glow = exp(-radius * 8.0) * 0.5;
  let glow_color = vec3<f32>(1.0, 0.8, 0.6) * center_glow;

  return vec4<f32>(final_color + glow_color, 1.0);
}
