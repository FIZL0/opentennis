//! The window's last camera (P45): instead of a 2D camera drawing a full-screen UI node (and two more for the
//! persistence), a camera of our own render schedule [`Out`] with one system, `draw`. It records this frame's
//! persistence mix into `last[i]` and the composite of that (or of the bare scene) with the HUD onto the window,
//! two passes in one command buffer. A Core2d camera costs a schedule run and up to four passes (clear, sprites,
//! UI, upscaling blit), each its own command buffer; the three it replaces were most of a frame's queue submit.

use bevy::core_pipeline::FullscreenShader;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::render::camera::ExtractedCamera;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::binding_types::{sampler, texture_2d, uniform_buffer};
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy::render::texture::GpuImage;
use bevy::render::view::{ExtractedWindows, ViewTargetAttachments};
use bevy::render::{GpuResourceAppExt, Render, RenderApp, RenderSystems};

/// The window camera's render schedule.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Out;

/// What `draw` puts on the window this frame, set by `hud_gamma::persist`.
#[derive(Resource, Clone, ExtractResource)]
pub struct Frame {
    pub scene: Handle<Image>,
    pub hud: Handle<Image>,
    pub last: [Handle<Image>; 2],
    /// In a match: this frame's parity `i` (mix the scene and `last[1 - i]` into `last[i]`, show that) and the mix's
    /// weights (x: how much of the last frame; y: the sun glare's FIX, 0 = none; zw: its texel in uv).
    pub mix: Option<(usize, Vec4)>,
    /// B629: the picture's rect in the window (uv: x, y, width, height) and how bright the bars beside it glow with
    /// its edges (0: black).
    pub picture: (Vec4, f32),
    /// B629: a small image the picture is averaged down into, the bars' glow blurred from it (while there are bars).
    pub glow: Handle<Image>,
}

/// B629: the glow image's side: each of its texels averages a 48th of the picture each way.
pub const GLOW_SIZE: u32 = 48;

pub fn plugin(app: &mut App) {
    app.add_plugins(ExtractResourcePlugin::<Frame>::default());
    let Some(r) = app.get_sub_app_mut(RenderApp) else { return };
    let mut schedule = Schedule::new(Out);
    schedule.set_build_settings(bevy::ecs::schedule::ScheduleBuildSettings { auto_insert_apply_deferred: false, ..default() });
    r.add_schedule(schedule)
        .add_systems(Out, draw)
        .init_gpu_resource::<Pipeline>()
        .init_gpu_resource::<SpecializedRenderPipelines<Pipeline>>()
        .init_resource::<Ids>()
        .add_systems(Render, prepare.in_set(RenderSystems::Prepare));
}

/// The window camera: drawn by [`Out`], last (after the HUD's 50).
pub fn camera() -> impl Bundle {
    // `Projection` only so Bevy computes its target size (`camera_system`); a layer nothing is on, as nothing is drawn
    (Camera { order: 51, ..default() }, bevy::render::camera::CameraRenderGraph::new(Out), Projection::default(), bevy::camera::visibility::RenderLayers::layer(super::LAYER))
}

#[derive(Resource)]
pub struct Pipeline {
    mix: BindGroupLayoutDescriptor,
    composite: BindGroupLayoutDescriptor,
    /// B629: the picture averaged into the glow image
    glow: BindGroupLayoutDescriptor,
    weight: Buffer,
    /// B629: the composite's `Frame::picture`
    picture: Buffer,
    fullscreen: FullscreenShader,
    shader: Handle<Shader>,
}

impl FromWorld for Pipeline {
    fn from_world(world: &mut World) -> Self {
        let tex = || texture_2d(TextureSampleType::Float { filterable: true });
        let mix = BindGroupLayoutDescriptor::new(
            "hst_persist",
            &BindGroupLayoutEntries::sequential(ShaderStages::FRAGMENT, (tex(), tex(), uniform_buffer::<Vec4>(false), sampler(SamplerBindingType::Filtering))),
        );
        let composite = BindGroupLayoutDescriptor::new(
            "hst_composite",
            &BindGroupLayoutEntries::sequential(ShaderStages::FRAGMENT, (tex(), tex(), sampler(SamplerBindingType::Filtering), uniform_buffer::<[Vec4; 2]>(false), tex(), sampler(SamplerBindingType::Filtering))),
        );
        let glow = BindGroupLayoutDescriptor::new("hst_glow", &BindGroupLayoutEntries::sequential(ShaderStages::FRAGMENT, (tex(), sampler(SamplerBindingType::Filtering))));
        let buffer = |label, size| world.resource::<RenderDevice>().create_buffer(&BufferDescriptor { label: Some(label), size, usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST, mapped_at_creation: false });
        let (weight, picture) = (buffer("hst_persist_weight", 16), buffer("hst_composite_picture", 32));
        let shader = world.resource::<AssetServer>().load("embedded://hst_base/hud_gamma.wgsl");
        Pipeline { mix, composite, glow, weight, picture, fullscreen: world.resource::<FullscreenShader>().clone(), shader }
    }
}

/// The window's passes.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pass {
    /// The persistence mix.
    Mix,
    /// B629: the picture averaged into the glow image.
    Glow,
    /// The composite onto the window.
    Composite,
}

impl SpecializedRenderPipeline for Pipeline {
    /// The target's format and the pass.
    type Key = (TextureFormat, Pass);

    fn specialize(&self, (format, pass): Self::Key) -> RenderPipelineDescriptor {
        let (label, layout, defs) = match pass {
            Pass::Mix => ("hst_persist", &self.mix, vec!["PERSIST".into()]),
            Pass::Glow => ("hst_glow", &self.glow, vec!["GLOW".into()]),
            Pass::Composite => ("hst_composite", &self.composite, vec![]),
        };
        RenderPipelineDescriptor {
            label: Some(label.into()),
            layout: vec![layout.clone()],
            vertex: self.fullscreen.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs: defs,
                entry_point: Some("fragment".into()),
                targets: vec![Some(ColorTargetState { format, blend: None, write_mask: ColorWrites::ALL })],
            }),
            ..default()
        }
    }
}

/// The mix's and the composite's pipelines for this frame's window format.
#[derive(Resource, Default)]
struct Ids {
    mix: Option<CachedRenderPipelineId>,
    glow: Option<CachedRenderPipelineId>,
    composite: Option<(TextureFormat, CachedRenderPipelineId)>,
}

fn prepare(
    frame: Option<Res<Frame>>,
    windows: Res<ExtractedWindows>,
    pipeline: Res<Pipeline>,
    cache: Res<PipelineCache>,
    queue: Res<RenderQueue>,
    mut pipelines: ResMut<SpecializedRenderPipelines<Pipeline>>,
    mut ids: ResMut<Ids>,
) {
    if ids.mix.is_none() {
        ids.mix = Some(pipelines.specialize(&cache, &pipeline, (TextureFormat::Rgba8Unorm, Pass::Mix)));
        ids.glow = Some(pipelines.specialize(&cache, &pipeline, (TextureFormat::Rgba8Unorm, Pass::Glow)));
    }
    let format = windows.primary.and_then(|w| windows.windows.get(&w)).and_then(|w| w.swap_chain_texture_view_format);
    if let Some(format) = format
        && ids.composite.is_none_or(|(f, _)| f != format)
    {
        ids.composite = Some((format, pipelines.specialize(&cache, &pipeline, (format, Pass::Composite))));
    }
    let Some(frame) = frame else { return };
    if let Some((_, w)) = frame.mix {
        let bytes: Vec<u8> = w.to_array().iter().flat_map(|f| f.to_le_bytes()).collect();
        queue.write_buffer(&pipeline.weight, 0, &bytes);
    }
    let (rect, glow) = frame.picture;
    let bytes: Vec<u8> = rect.to_array().into_iter().chain([glow, 0.0, 0.0, 0.0]).flat_map(|f| f.to_le_bytes()).collect();
    queue.write_buffer(&pipeline.picture, 0, &bytes);
}

/// Bind groups kept while the texture views they hold stay the same (they change when an image is resized).
#[derive(Default)]
struct Groups {
    mix: [Option<([TextureViewId; 2], SamplerId, BindGroup)>; 2],
    composite: Option<([TextureViewId; 3], SamplerId, BindGroup)>,
    glow: Option<(TextureViewId, SamplerId, BindGroup)>,
}

#[allow(clippy::too_many_arguments)]
fn draw(
    view: ViewQuery<&ExtractedCamera>,
    frame: Option<Res<Frame>>,
    attachments: Res<ViewTargetAttachments>,
    images: Res<RenderAssets<GpuImage>>,
    pipeline: Res<Pipeline>,
    ids: Res<Ids>,
    cache: Res<PipelineCache>,
    mut groups: Local<Groups>,
    mut ctx: RenderContext,
) {
    let camera = view.into_inner();
    // the window's output, or the capture texture standing in for it while a screenshot is taken
    let Some(out) = camera.target.as_ref().and_then(|t| attachments.get(t)) else { return };
    let Some(frame) = frame else { return };
    let device = ctx.render_device().clone();
    // a bind group kept while its texture views (`key`) and sampler stay the same
    fn kept<K: PartialEq + Copy>(slot: &mut Option<(K, SamplerId, BindGroup)>, key: K, sampler: SamplerId, make: impl FnOnce() -> BindGroup) -> BindGroup {
        if !slot.as_ref().is_some_and(|(k, s, _)| *k == key && *s == sampler) {
            *slot = Some((key, sampler, make()));
        }
        slot.as_ref().unwrap().2.clone()
    }
    let (Some(scene), Some(hud), Some(glow)) = (images.get(&frame.scene), images.get(&frame.hud), images.get(&frame.glow)) else { return };
    let pass_into = |ctx: &mut RenderContext, label, view: &TextureView, p: &RenderPipeline, g: &BindGroup| {
        let mut pass = ctx.command_encoder().begin_render_pass(&RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(RenderPassColorAttachment { view, depth_slice: None, resolve_target: None, ops: Operations { load: LoadOp::Clear(default()), store: StoreOp::Store } })],
            ..default()
        });
        pass.set_pipeline(p);
        pass.set_bind_group(0, g, &[]);
        pass.draw(0..3, 0..1);
    };
    // this frame's persistence: the scene and the last frame shown into `last[i]`
    let mut shown = scene;
    if let Some((i, _)) = frame.mix
        && let (Some(dst), Some(prev)) = (images.get(&frame.last[i]), images.get(&frame.last[1 - i]))
        && let Some(p) = ids.mix.and_then(|id| cache.get_render_pipeline(id))
    {
        let g = kept(&mut groups.mix[i], [scene.texture_view.id(), prev.texture_view.id()], scene.sampler.id(), || {
            let layout = cache.get_bind_group_layout(&pipeline.mix);
            device.create_bind_group(None, &layout, &BindGroupEntries::sequential((&scene.texture_view, &prev.texture_view, pipeline.weight.as_entire_binding(), &scene.sampler)))
        });
        pass_into(&mut ctx, "hst_persist", &dst.texture_view, p, &g);
        shown = dst;
    }
    // B629: the picture averaged into the glow image, while there are bars to glow
    let (rect, strength) = frame.picture;
    if strength > 0.0
        && rect != Vec4::new(0.0, 0.0, 1.0, 1.0)
        && let Some(p) = ids.glow.and_then(|id| cache.get_render_pipeline(id))
    {
        let g = kept(&mut groups.glow, shown.texture_view.id(), shown.sampler.id(), || {
            let layout = cache.get_bind_group_layout(&pipeline.glow);
            device.create_bind_group(None, &layout, &BindGroupEntries::sequential((&shown.texture_view, &shown.sampler)))
        });
        pass_into(&mut ctx, "hst_glow", &glow.texture_view, p, &g);
    }
    // the composite, on the whole window (B629: the picture in its rect, B117's fixed aspect ratios' bars around it);
    // the window is cleared even while the pipeline compiles
    let p = ids.composite.and_then(|(_, id)| cache.get_render_pipeline(id));
    let g = kept(&mut groups.composite, [shown.texture_view.id(), hud.texture_view.id(), glow.texture_view.id()], shown.sampler.id(), || {
        let layout = cache.get_bind_group_layout(&pipeline.composite);
        let entries = (&shown.texture_view, &hud.texture_view, &shown.sampler, pipeline.picture.as_entire_binding(), &glow.texture_view, &glow.sampler);
        device.create_bind_group(None, &layout, &BindGroupEntries::sequential(entries))
    });
    let mut pass = ctx.command_encoder().begin_render_pass(&RenderPassDescriptor {
        label: Some("hst_composite"),
        color_attachments: &[Some(out.get_attachment(Some(LinearRgba::BLACK)))],
        ..default()
    });
    let Some(p) = p else { return };
    pass.set_pipeline(p);
    pass.set_bind_group(0, &g, &[]);
    pass.draw(0..3, 0..1);
}
