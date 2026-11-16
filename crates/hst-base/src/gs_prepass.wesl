// Shadow pass for court models (see gs.rs): the same alpha test as gs.wesl, so cut-out leaves cast leaf shadows.
import bevy_pbr::prepass::io::{Vertex, UncompressedVertex, VertexOutput, decompress_vertex};
import bevy_pbr::render::{mesh_bindings::mesh, mesh_functions, skinning, morph, view_transformations::position_world_to_clip};
@if(PREPASS_FRAGMENT)
import bevy_pbr::prepass::io::FragmentOutput;

struct Gs {
    color: vec4<f32>,
    shininess: f32,
    highlight: f32,
    shadow: f32,
    uv_offset: vec2<f32>,
    fog: vec4<f32>,
    fog_color: vec4<f32>,
    lod_k: f32,
    // the rest of gs.wesl's, up to its flags (gs.rs `FLAG_*`) and alpha test bounds
    light_dir: vec4<f32>,
    light_color: vec4<f32>,
    ambient: vec4<f32>,
    light2_dir: vec4<f32>,
    light2_color: vec4<f32>,
    glare: f32,
    unlit: f32,
    screen_lod: f32,
    flags: u32,
    alpha_min: f32,
    alpha_max: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> gs: Gs;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var gs_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var gs_sampler: sampler;

// as gs.wesl's (positions only)
@if(MORPH_TARGETS)
fn morph_vertex(vertex_in: UncompressedVertex, instance_index: u32) -> UncompressedVertex {
    var vertex = vertex_in;
    let vertex_index = vertex.index - mesh[instance_index].first_vertex_index;
    let descriptor = mesh[instance_index].morph_descriptor_index;
    @if(!SKINS_USE_UNIFORM_BUFFERS)
    if descriptor == 0xffffffffu {
        return vertex;
    }
    for (var i = 0u; i < morph::layer_count(descriptor); i++) {
        let weight = morph::weight_at(i, descriptor);
        if weight != 0.0 {
            vertex.position += weight * morph::morph_position(vertex_index, i, descriptor);
        }
    }
    return vertex;
}

// Bevy's prepass vertex (prepass.wesl) for what the court models use, no motion vectors
@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let instance = vertex_in.instance_index;
    let vertex_no_morph = decompress_vertex(vertex_in, instance);
    @if(MORPH_TARGETS)
    let vertex = morph_vertex(vertex_no_morph, instance);
    @else
    let vertex = vertex_no_morph;
    @if(SKINNED)
    let world_from_local = skinning::skin_model(vertex.joint_indices, vertex.joint_weights, instance);
    @else
    let world_from_local = mesh_functions::get_world_from_local(instance);
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(vertex.position, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
    @if(UNCLIPPED_DEPTH_ORTHO_EMULATION) {
    out.unclipped_depth = out.position.z;
    out.position.z = min(out.position.z, 1.0);
    }
    @if(VERTEX_UVS_A)
    out.uv = vertex.uv;
    @if(VERTEX_COLORS)
    out.color = vertex.color;
    @if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = instance;
    @if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(instance, mesh_functions::get_world_from_local(instance)[3]);
    return out;
}

fn alpha_test(in: VertexOutput) {
    // no Z written: casts no shadow
    if (gs.flags & 32u) != 0u {
        discard;
    }
    @if(VERTEX_COLORS)
    var a = min(in.color.a * gs.color.a, 255.0 / 128.0);
    @else
    var a = min(gs.color.a, 255.0 / 128.0);
    @if(VERTEX_UVS_A)
    if (gs.flags & 1u) != 0u {
        // the sun sees the top level (the GS picks levels from the camera's depth, gs.wesl)
        let t = textureSampleLevel(gs_texture, gs_sampler, in.uv + gs.uv_offset, 0.0).a;
        a = select(t, t * a, (gs.flags & 2u) != 0u);
    }
    if floor(a * 128.0) < gs.alpha_min { discard; }
}

@if(PREPASS_FRAGMENT)
@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    alpha_test(in);
    var out: FragmentOutput;
    @if(UNCLIPPED_DEPTH_ORTHO_EMULATION)
    out.frag_depth = in.unclipped_depth;
    return out;
}

@if(!PREPASS_FRAGMENT)
@fragment
fn fragment(in: VertexOutput) {
    alpha_test(in);
}
