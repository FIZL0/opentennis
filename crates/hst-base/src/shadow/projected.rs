//! G2: Shadows Original. The PS2 has no shadow map: each caster gets its own 128² 4-bit shadow texture at load
//! (`hst_sim::shade::texture`), framed on the caster's own extent across the light, and the ground triangles inside
//! its shadow volume (`hst_sim::shade::receiver_tris`) are drawn again with it projected along the sun, multiplying
//! the frame buffer by 1 − darken × texel (GS ALPHA (0 − Cd)·A + Cd). So a 30 m tree's shadow is a soft
//! 25 cm-a-texel blob and a seat's a sharp 2 cm one, which no single shadow map gives.
//!
//! Here the props' shadows are drawn that way: one draw per caster (its receiver triangles in game space under the
//! court root, its texture's (s, t) per vertex), after the hole's own draws, depth-tested against the ground. While
//! they're shown, the props leave the sun's shadow map (`shadow::low`); the players stay in it (the game's dynamic
//! casters, ~2 cm a texel there, close to the map's).
//!
//! ponytail: the texel's darkening is its red (index·0x20, clamped) / 255 × the hole's darkening, the CLUT the
//! game's shade map reads; the visible pass's own CLUT wasn't captured.

use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::{MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, DepthBiasState, Extent3d,
    RenderPipelineDescriptor, SpecializedMeshPipelineError, TextureDimension, TextureFormat,
};
use bevy::shader::ShaderRef;
use hst_sim::shade::{self, Caster};

use crate::graphics::{Graphics, Shadows};

/// The root of a court's projected shadows (a child of the game-space court root): shown at Shadows Original.
#[derive(Component)]
pub struct Projected;

/// One caster's shadow draw: its texture and the darkening (x) it takes off the ground at a full texel.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct ProjectedMaterial {
    #[uniform(0)]
    pub darken: Vec4,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
}

/// Sorted after the hole's draws (−1e6 + 1e3 per material, `main::models`) and before everything standing on it.
const ORDER: f32 = -5.0e5;

impl Material for ProjectedMaterial {
    fn vertex_shader() -> ShaderRef {
        "embedded://hst_base/shadow/projected.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "embedded://hst_base/shadow/projected.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Multiply
    }

    fn depth_bias(&self) -> f32 {
        ORDER
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // strip triangles of either winding, as the GS draws them
        descriptor.primitive.cull_mode = None;
        // the same ground triangles as the hole's, through another shader: pulled a little towards the eye (reversed Z)
        if let Some(ds) = descriptor.depth_stencil.as_mut() {
            ds.depth_write_enabled = Some(false);
            ds.bias = DepthBiasState { constant: 16, slope_scale: 1.0, clamp: 0.0 };
        }
        // Cd × Cs: the shader's colour is 1 − darken × texel; the target's alpha kept
        let blend = BlendState {
            color: BlendComponent { src_factor: BlendFactor::Dst, dst_factor: BlendFactor::Zero, operation: BlendOperation::Add },
            alpha: BlendComponent { src_factor: BlendFactor::Zero, dst_factor: BlendFactor::One, operation: BlendOperation::Add },
        };
        for t in descriptor.fragment.iter_mut().flat_map(|f| f.targets.iter_mut().flatten()) {
            t.blend = Some(blend);
        }
        Ok(())
    }
}

pub fn plugin(app: &mut App) {
    bevy::asset::embedded_asset!(app, "projected.wgsl");
    app.add_plugins(MaterialPlugin::<ProjectedMaterial>::default()).add_systems(Update, show.after(crate::weather::apply));
}

/// A caster's shadow texture as an image: each texel's red (index·0x20, clamped), filtered bilinear, clamped.
fn image(c: &Caster) -> Image {
    let data = shade::texture(c).iter().map(|&i| (u32::from(i) * 0x20).min(0xff) as u8).collect();
    let n = shade::TEX as u32;
    let mut img = Image::new(Extent3d { width: n, height: n, depth_or_array_layers: 1 }, TextureDimension::D2, data, TextureFormat::R8Unorm, default());
    img.sampler = bevy::image::ImageSampler::linear();
    img
}

/// A caster's receiver triangles (game space) with its texture's (s, t) at each corner: the light matrix's (u, v)
/// over w, taken from −1..1 to 0..1. `None` when no ground lies in its shadow volume.
fn mesh(c: &Caster, hole: &hst_data::mdl::Model, mats: &hst_data::mtl::Mtl) -> Option<Mesh> {
    let tris = shade::receiver_tris(&c.planes, hole, mats);
    if tris.is_empty() {
        return None;
    }
    let l = &c.light;
    let st = |p: [f32; 3]| {
        let r = |j: usize| p[0] * l[0][j] + p[1] * l[1][j] + p[2] * l[2][j] + l[3][j];
        let w = r(3);
        [0.5 * r(0) / w + 0.5, 0.5 * r(1) / w + 0.5]
    };
    let pos: Vec<[f32; 3]> = tris.into_iter().flatten().collect();
    let uv: Vec<[f32; 2]> = pos.iter().map(|&p| st(p)).collect();
    Some(
        Mesh::new(PrimitiveTopology::TriangleList, default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv),
    )
}

/// Build the court's projected shadows under `root` (game space) from its static `casters` and the `hole` model:
/// the textures and meshes now, the draws once the commands run.
pub fn spawn(commands: &mut Commands, root: Entity, casters: &[Caster], hole: &hst_data::mdl::Model, mats: &hst_data::mtl::Mtl) {
    let draws: Vec<(Mesh, Image)> = casters.iter().filter_map(|c| Some((mesh(c, hole, mats)?, image(c)))).collect();
    commands.queue(move |world: &mut World| {
        if world.get_entity(root).is_err() {
            return;
        }
        let shown = world.get_resource::<Graphics>().is_some_and(|g| g.shadows_drawn() == Shadows::Original);
        let parent = world.spawn((Projected, Transform::default(), if shown { Visibility::Inherited } else { Visibility::Hidden }, ChildOf(root))).id();
        for (m, img) in draws {
            let m = world.resource_mut::<Assets<Mesh>>().add(m);
            let texture = world.resource_mut::<Assets<Image>>().add(img);
            let mat = world.resource_mut::<Assets<ProjectedMaterial>>().add(ProjectedMaterial { darken: Vec4::ZERO, texture });
            world.spawn((Mesh3d(m), MeshMaterial3d(mat), NotShadowCaster, NotShadowReceiver, Transform::default(), ChildOf(parent)));
        }
    });
}

/// Shown at Shadows Original, darkening the ground by as much as the hole's shadow does (its weather included).
fn show(
    g: Res<Graphics>,
    look: Option<Res<crate::weather::CourtLook>>,
    gs: Res<Assets<crate::gs::GsMaterial>>,
    mut roots: Query<&mut Visibility, With<Projected>>,
    draws: Query<&MeshMaterial3d<ProjectedMaterial>>,
    mut materials: ResMut<Assets<ProjectedMaterial>>,
) {
    if roots.is_empty() {
        return;
    }
    let shown = g.shadows_drawn() == Shadows::Original;
    for mut v in &mut roots {
        v.set_if_neq(if shown { Visibility::Inherited } else { Visibility::Hidden });
    }
    let Some(darken) = look.and_then(|l| l.holes.iter().find_map(|id| gs.get(*id)).map(|m| m.uniform.shadow)) else { return };
    for d in &draws {
        if materials.get(&d.0).is_some_and(|m| m.darken.x != darken) {
            materials.get_mut(&d.0).unwrap().darken.x = darken;
        }
    }
}
