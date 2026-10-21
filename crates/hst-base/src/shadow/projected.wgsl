// G2: a caster's shadow texture projected on its receiver triangles (projected.rs): the frame buffer × this colour.
#import bevy_pbr::mesh_functions
#import bevy_pbr::view_transformations::position_world_to_clip

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> darken: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var shadow_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var shadow_sampler: sampler;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(vertex.position, 1.0));
    out.position = position_world_to_clip(world.xyz);
    out.uv = vertex.uv;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // the GS: (0 − Cd)·A + Cd, A the bilinear texel × the strength
    let a = darken.x * textureSample(shadow_texture, shadow_sampler, in.uv).r;
    return vec4(vec3(1.0 - a), 1.0);
}
