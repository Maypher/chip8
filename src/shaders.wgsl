struct VertexOutput {
  @builtin(position) pos: vec4<f32>,
  @location(0) @interpolate(flat) on: f32,
};

@group(0) @binding(0)
var<uniform> ortho_matrix: mat4x4<f32>;

@vertex
fn vs_main(@location(0) vpos: vec2<f32>, @location(1) ipos: vec2<f32>, @location(2) on: f32) -> VertexOutput {
  // vpos is the vertex position, ipos is the instance position, on is whether or not this tile is illuminated

  var output: VertexOutput;

  let world_width = 64.0;
  let world_height = 32.0;

  let world_x = vpos.x + ipos.x;
  let world_y = vpos.y + ipos.y;

  let x = world_x / world_width * 2.0 - 1.0;
  let y = world_y / world_height * 2.0 - 1.0;

  output.pos = vec4<f32>(x, y, 0.0, 1.0);
  output.on = on;

  return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  // the colour will be on for each rgb value: if on is zero it should be black, and if on is 1 it should be white perfect
  var c: f32 = f32(input.on);

  return vec4<f32>(c, c, c, 1.0);
}