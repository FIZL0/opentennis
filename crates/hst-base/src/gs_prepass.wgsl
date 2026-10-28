// Shadow pass for court models (see gs.rs): the same alpha test as gs.wgsl, so cut-out leaves cast leaf shadows.
#import bevy_pbr::prepass_io::{Vertex, VertexOutput}
#import bevy_pbr::{mesh_bindings::mesh, mesh_functions, skinning, morph}
#import bevy_pbr::view_transformations::position_world_to_clip
#ifdef PREPASS_FRAGMENT
#import bevy_pbr::prepass_io::FragmentOutput
#endif

struct Gs {
    color: vec4<f32>,
    shininess: f32,
    highlight: f32,
    shadow: f32,
    uv_offset: vec2<f32>,
    fog: vec4<f32>,
    fog_color: vec4<f32>,
    lod_k: f32,
    // the rest of gs.wgsl's, up to its flags (gs.rs `FLAG_*`) and alpha test bounds
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

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> gs: Gs;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var gs_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var gs_sampler: sampler;

#ifdef MORPH_TARGETS
// as gs.wgsl's (positions only)
fn morph_vertex(vertex_in: Vertex, instance_index: u32) -> Vertex {
    var vertex = vertex_in;
    let vertex_index = vertex.index - mesh[instance_index].first_vertex_index;
#ifndef SKINS_USE_UNIFORM_BUFFERS
    if mesh[instance_index].morph_descriptor_index == 0xffffffffu {
        return vertex;
    }
#endif
    for (var i = 0u; i < morph::layer_count(instance_index); i++) {
        let weight = morph::weight_at(i, instance_index);
        if weight != 0.0 {
            vertex.position += weight * morph::morph_position(vertex_index, i, instance_index);
        }
    }
    return vertex;
}
#endif

// Bevy's prepass vertex (prepass.wgsl) for what the court models use, no motion vectors
@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let instance = vertex_no_morph.instance_index;
#ifdef MORPH_TARGETS
    let vertex = morph_vertex(vertex_no_morph, instance);
#else
    let vertex = vertex_no_morph;
#endif
#ifdef SKINNED
    let world_from_local = skinning::skin_model(vertex.joint_indices, vertex.joint_weights, instance);
#else
    let world_from_local = mesh_functions::get_world_from_local(instance);
#endif
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(vertex.position, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    out.unclipped_depth = out.position.z;
    out.position.z = min(out.position.z, 1.0);
#endif
#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = instance;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(instance, mesh_functions::get_world_from_local(instance)[3]);
#endif
    return out;
}

fn alpha_test(in: VertexOutput) {
    // no Z written: casts no shadow
    if (gs.flags & 32u) != 0u {
        discard;
    }
#ifdef VERTEX_COLORS
    var a = min(in.color.a * gs.color.a, 255.0 / 128.0);
#else
    var a = min(gs.color.a, 255.0 / 128.0);
#endif
#ifdef VERTEX_UVS_A
    if (gs.flags & 1u) != 0u {
        // the sun sees the top level (the GS picks levels from the camera's depth, gs.wgsl)
        let t = textureSampleLevel(gs_texture, gs_sampler, in.uv + gs.uv_offset, 0.0).a;
        a = select(t, t * a, (gs.flags & 2u) != 0u);
    }
#endif
    if floor(a * 128.0) < gs.alpha_min { discard; }
}

#ifdef PREPASS_FRAGMENT
@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    alpha_test(in);
    var out: FragmentOutput;
#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    out.frag_depth = in.unclipped_depth;
#endif
    return out;
}
#else
@fragment
fn fragment(in: VertexOutput) {
    alpha_test(in);
}
#endif
