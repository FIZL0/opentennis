use bevy_material::descriptor::{
    BindGroupLayoutDescriptor, CachedComputePipelineId, ComputePipelineDescriptor,
    MeshPipelineDescriptor, PipelineDescriptor, RenderPipelineDescriptor,
};
use smallvec::SmallVec;

use crate::{
    render_resource::*,
    renderer::{wgpu_wrapper, RenderDevice},
    Extract,
};
use alloc::{borrow::Cow, sync::Arc};
use bevy_asset::{AssetEvent, AssetId, Assets, Handle};
use bevy_ecs::{
    message::MessageReader,
    resource::Resource,
    system::{Res, ResMut},
};
use bevy_log::error;
use bevy_platform::collections::{hash_map::RawEntryMut, HashMap, HashSet};
use bevy_shader::{
    CachedPipelineId, Shader, ShaderCache, ShaderCacheError, ShaderCacheSource, ShaderDefVal,
    ValidateShader,
};
use bevy_tasks::Task;
use bevy_utils::default;
use core::{future::Future, mem};
use std::sync::{Mutex, PoisonError};
use wgpu::{PipelineCompilationOptions, VertexBufferLayout as RawVertexBufferLayout};

/// A pipeline defining the data layout and shader logic for a specific GPU task.
///
/// Used to store a heterogenous collection of render and compute pipelines together.
#[derive(Debug)]
pub enum Pipeline {
    RenderPipeline(RenderPipeline),
    ComputePipeline(ComputePipeline),
    MeshPipeline(RenderPipeline),
}

pub struct CachedPipeline {
    pub descriptor: PipelineDescriptor,
    pub state: CachedPipelineState,
}

/// State of a cached pipeline inserted into a [`PipelineCache`].
#[derive(Debug)]
pub enum CachedPipelineState {
    /// The pipeline GPU object is queued for creation.
    Queued,
    /// The pipeline GPU object is being created.
    Creating(Task<Result<Pipeline, ShaderCacheError>>),
    /// The pipeline GPU object was created successfully and is available (allocated on the GPU).
    Ok(Pipeline),
    /// An error occurred while trying to create the pipeline GPU object.
    Err(ShaderCacheError),
}

impl CachedPipelineState {
    /// Convenience method to "unwrap" a pipeline state into its underlying GPU object.
    ///
    /// # Returns
    ///
    /// The method returns the allocated pipeline GPU object.
    ///
    /// # Panics
    ///
    /// This method panics if the pipeline GPU object is not available, either because it is
    /// pending creation or because an error occurred while attempting to create GPU object.
    pub fn unwrap(&self) -> &Pipeline {
        match self {
            CachedPipelineState::Ok(pipeline) => pipeline,
            CachedPipelineState::Queued => {
                panic!("Pipeline has not been compiled yet. It is still in the 'Queued' state.")
            }
            CachedPipelineState::Creating(..) => {
                panic!("Pipeline has not been compiled yet. It is still in the 'Creating' state.")
            }
            CachedPipelineState::Err(err) => panic!("{}", err),
        }
    }
}

// The default webgpu `max_bind_groups` is 4. Most desktop GPUs support 8.
// Since the element size we used below is small, we inline 8 on the stack.
const BIND_GROUP_LAYOUTS_INLINE_CAPACITY: usize = 8;

type ImmediateSize = u32;
type LayoutCacheKey = (
    SmallVec<[BindGroupLayoutId; BIND_GROUP_LAYOUTS_INLINE_CAPACITY]>,
    ImmediateSize,
);

wgpu_wrapper! {
    struct WgpuPipelineLayout(PipelineLayout);

    struct WgpuShaderModule(ShaderModule);
}

#[derive(Default)]
struct LayoutCache {
    layouts: HashMap<LayoutCacheKey, Arc<WgpuPipelineLayout>>,
}

impl LayoutCache {
    fn get(
        &mut self,
        render_device: &RenderDevice,
        bind_group_layouts: &[BindGroupLayout],
        immediate_size: u32,
    ) -> Arc<WgpuPipelineLayout> {
        let bind_group_ids = bind_group_layouts.iter().map(BindGroupLayout::id).collect();
        self.layouts
            .entry((bind_group_ids, immediate_size))
            .or_insert_with_key(|(_, immediate_size)| {
                let bind_group_layouts = bind_group_layouts
                    .iter()
                    .map(BindGroupLayout::value)
                    .map(Some)
                    .collect::<SmallVec<[_; BIND_GROUP_LAYOUTS_INLINE_CAPACITY]>>();
                Arc::new(WgpuPipelineLayout::new(
                    render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
                        bind_group_layouts: &bind_group_layouts,
                        immediate_size: *immediate_size,
                        ..default()
                    }),
                ))
            })
            .clone()
    }
}

fn load_module(
    render_device: &RenderDevice,
    shader_source: ShaderCacheSource,
    validate_shader: &ValidateShader,
) -> Result<WgpuShaderModule, ShaderCacheError> {
    let shader_source = match shader_source {
        #[cfg(feature = "shader_format_spirv")]
        ShaderCacheSource::SpirV(data) => wgpu::util::make_spirv(data),
        #[cfg(not(feature = "shader_format_spirv"))]
        ShaderCacheSource::SpirV(_) => {
            unimplemented!("Enable feature \"shader_format_spirv\" to use SPIR-V shaders")
        }
        ShaderCacheSource::Wgsl(src) => ShaderSource::Wgsl(Cow::Owned(src)),
    };
    let module_descriptor = ShaderModuleDescriptor {
        label: None,
        source: shader_source,
    };

    let scope = render_device
        .wgpu_device()
        .push_error_scope(wgpu::ErrorFilter::Validation);

    let shader_module = WgpuShaderModule::new(match validate_shader {
        ValidateShader::Enabled => {
            render_device.create_and_validate_shader_module(module_descriptor)
        }
        // SAFETY: we are interfacing with shader code, which may contain undefined behavior,
        // such as indexing out of bounds.
        // The checks required are prohibitively expensive and a poor default for game engines.
        ValidateShader::Disabled => unsafe {
            render_device.create_shader_module(module_descriptor)
        },
    });

    let error = scope.pop();

    // `now_or_never` will return Some if the future is ready and None otherwise.
    // On native platforms, wgpu will yield the error immediately while on wasm it may take longer since the browser APIs are asynchronous.
    // So to keep the complexity of the ShaderCache low, we will only catch this error early on native platforms,
    // and on wasm the error will be handled by wgpu and crash the application.
    if let Some(Some(wgpu::Error::Validation { description, .. })) =
        bevy_tasks::futures::now_or_never(error)
    {
        return Err(ShaderCacheError::CreateShaderModule(description));
    }

    Ok(shader_module)
}

#[derive(Default)]
struct BindGroupLayoutCache {
    bgls: HashMap<BindGroupLayoutDescriptor, BindGroupLayout>,
}

impl BindGroupLayoutCache {
    fn get(
        &mut self,
        render_device: &RenderDevice,
        descriptor: &BindGroupLayoutDescriptor,
    ) -> BindGroupLayout {
        match self.bgls.raw_entry_mut().from_key(descriptor) {
            RawEntryMut::Occupied(entry) => entry.into_mut(),
            RawEntryMut::Vacant(slot) => {
                let created = render_device
                    .create_bind_group_layout(descriptor.label.as_ref(), &descriptor.entries);
                slot.insert(descriptor.clone(), created).1
            }
        }
        .clone()
    }
}

/// Cache for render and compute pipelines.
///
/// The cache stores existing render and compute pipelines allocated on the GPU, as well as
/// pending creation. Pipelines inserted into the cache are identified by a unique ID, which
/// can be used to retrieve the actual GPU object once it's ready. The creation of the GPU
/// pipeline object is deferred to the [`RenderSystems::Render`] step, just before the render
/// graph starts being processed, as this requires access to the GPU.
///
/// Note that the cache does not perform automatic deduplication of identical pipelines. It is
/// up to the user not to insert the same pipeline twice to avoid wasting GPU resources.
///
/// [`RenderSystems::Render`]: crate::RenderSystems::Render
#[derive(Resource)]
pub struct PipelineCache {
    layout_cache: Arc<Mutex<LayoutCache>>,
    bindgroup_layout_cache: Arc<Mutex<BindGroupLayoutCache>>,
    shader_cache: Arc<Mutex<ShaderCache<WgpuShaderModule, RenderDevice>>>,
    device: RenderDevice,
    pipelines: Vec<CachedPipeline>,
    waiting_pipelines: HashSet<CachedPipelineId>,
    new_pipelines: Mutex<Vec<CachedPipeline>>,
    global_shader_defs: Vec<ShaderDefVal>,
    /// If `true`, disables asynchronous pipeline compilation.
    /// This has no effect on macOS, wasm, or without the `multi_threaded` feature.
    pub(crate) synchronous_pipeline_compilation: bool,
    /// If `true`, the shader cache needs to be repopulated from the main world's `Assets<Shader>`.
    needs_shader_reload: bool,
    /// opentennis P47: when pipelines compile on this thread (macOS, the browser, or `synchronous_pipeline_compilation`),
    /// a queue pass stops starting new ones once this much time went by; the rest stay queued for the next frame. At
    /// least one is started each pass. `None`: Bevy's own behaviour, the whole queue at once.
    compile_budget: Option<core::time::Duration>,
    /// opentennis P55: render pipelines by a hash of their descriptor, so queueing one equal to a pipeline already in
    /// the cache returns that one's id instead of compiling it again; and those queued only by
    /// [`Self::prewarm_render_pipeline`] that nothing has asked for since.
    dedup: Mutex<Dedup>,
}

/// opentennis P55: see [`PipelineCache::dedup`].
#[derive(Default)]
struct Dedup {
    by_hash: HashMap<u64, Vec<usize>>,
    unrequested: HashSet<usize>,
}

/// opentennis P55: the hash [`PipelineCache::dedup`] files a render pipeline under (the fields that tell most apart).
fn descriptor_hash(d: &RenderPipelineDescriptor) -> u64 {
    use core::hash::{Hash, Hasher};
    let mut h = std::hash::DefaultHasher::new();
    d.label.hash(&mut h);
    d.vertex.shader.id().hash(&mut h);
    d.vertex.shader_defs.hash(&mut h);
    d.vertex.buffers.hash(&mut h);
    if let Some(f) = &d.fragment {
        f.shader.id().hash(&mut h);
        f.shader_defs.hash(&mut h);
    }
    h.finish()
}

impl PipelineCache {
    /// Returns an iterator over the pipelines in the pipeline cache.
    pub fn pipelines(&self) -> impl Iterator<Item = &CachedPipeline> {
        self.pipelines.iter()
    }

    /// Returns a iterator of the IDs of all currently waiting pipelines.
    pub fn waiting_pipelines(&self) -> impl Iterator<Item = CachedPipelineId> + '_ {
        self.waiting_pipelines.iter().copied()
    }

    /// Create a new pipeline cache associated with the given render device.
    pub fn new(device: RenderDevice, synchronous_pipeline_compilation: bool) -> Self {
        let mut global_shader_defs = Vec::new();
        #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
        {
            global_shader_defs.push("NO_ARRAY_TEXTURES_SUPPORT".into());
            global_shader_defs.push("NO_CUBE_ARRAY_TEXTURES_SUPPORT".into());
            global_shader_defs.push("SIXTEEN_BYTE_ALIGNMENT".into());
        }

        // HST (M154): ARM Linux too (GL ES binds no cube arrays; the point-light shadow view is a Cube there)
        if cfg!(any(target_abi = "sim", all(target_os = "linux", target_arch = "aarch64"))) {
            global_shader_defs.push("NO_CUBE_ARRAY_TEXTURES_SUPPORT".into());
        }

        let available_storage_buffer_bindings =
            device.limits().max_storage_buffers_per_shader_stage;
        global_shader_defs.push(ShaderDefVal::UInt(
            "AVAILABLE_STORAGE_BUFFER_BINDINGS".into(),
            available_storage_buffer_bindings,
        ));
        // wesl condcomp flags are booleans, so bake the comparisons in.
        global_shader_defs.push(ShaderDefVal::Bool(
            "AVAILABLE_STORAGE_BUFFER_BINDINGS__GE_3".into(),
            available_storage_buffer_bindings >= 3,
        ));
        global_shader_defs.push(ShaderDefVal::Bool(
            "AVAILABLE_STORAGE_BUFFER_BINDINGS__GE_6".into(),
            available_storage_buffer_bindings >= 6,
        ));

        Self {
            shader_cache: Arc::new(Mutex::new(ShaderCache::new(device.clone(), load_module))),
            device,
            layout_cache: default(),
            bindgroup_layout_cache: default(),
            waiting_pipelines: default(),
            new_pipelines: default(),
            pipelines: default(),
            global_shader_defs,
            synchronous_pipeline_compilation,
            needs_shader_reload: true,
            compile_budget: None,
            dedup: default(),
        }
    }

    /// Get the state of a cached render pipeline.
    ///
    /// See [`PipelineCache::queue_render_pipeline()`].
    #[inline]
    pub fn get_render_pipeline_state(&self, id: CachedRenderPipelineId) -> &CachedPipelineState {
        // If the pipeline id isn't in `pipelines`, it's queued in `new_pipelines`
        self.pipelines
            .get(id.id())
            .map_or(&CachedPipelineState::Queued, |pipeline| &pipeline.state)
    }

    /// Get the state of a cached compute pipeline.
    ///
    /// See [`PipelineCache::queue_compute_pipeline()`].
    #[inline]
    pub fn get_compute_pipeline_state(&self, id: CachedComputePipelineId) -> &CachedPipelineState {
        // If the pipeline id isn't in `pipelines`, it's queued in `new_pipelines`
        self.pipelines
            .get(id.id())
            .map_or(&CachedPipelineState::Queued, |pipeline| &pipeline.state)
    }

    /// Get the render pipeline descriptor a cached render pipeline was inserted from.
    ///
    /// See [`PipelineCache::queue_render_pipeline()`].
    ///
    /// **Note**: Be careful calling this method. It will panic if called with a pipeline that
    /// has been queued but has not yet been processed by [`PipelineCache::process_queue()`].
    #[inline]
    pub fn get_render_pipeline_descriptor(
        &self,
        id: CachedRenderPipelineId,
    ) -> &RenderPipelineDescriptor {
        match &self.pipelines[id.id()].descriptor {
            PipelineDescriptor::RenderPipelineDescriptor(descriptor) => descriptor,
            PipelineDescriptor::ComputePipelineDescriptor(_)
            | PipelineDescriptor::MeshPipelineDescriptor(_) => unreachable!(),
        }
    }

    /// Get the compute pipeline descriptor a cached render pipeline was inserted from.
    ///
    /// See [`PipelineCache::queue_compute_pipeline()`].
    ///
    /// **Note**: Be careful calling this method. It will panic if called with a pipeline that
    /// has been queued but has not yet been processed by [`PipelineCache::process_queue()`].
    #[inline]
    pub fn get_compute_pipeline_descriptor(
        &self,
        id: CachedComputePipelineId,
    ) -> &ComputePipelineDescriptor {
        match &self.pipelines[id.id()].descriptor {
            PipelineDescriptor::RenderPipelineDescriptor(_)
            | PipelineDescriptor::MeshPipelineDescriptor(_) => unreachable!(),
            PipelineDescriptor::ComputePipelineDescriptor(descriptor) => descriptor,
        }
    }

    /// Get the mesh pipeline descriptor a cached mesh pipeline was inserted from.
    ///
    /// See [`PipelineCache::queue_mesh_pipeline()`].
    ///
    /// **Note**: Be careful calling this method. It will panic if called with a pipeline that
    /// has been queued but has not yet been processed by [`PipelineCache::process_queue()`].
    #[inline]
    pub fn get_mesh_pipeline_descriptor(
        &self,
        id: CachedRenderPipelineId,
    ) -> &MeshPipelineDescriptor {
        match &self.pipelines[id.id()].descriptor {
            PipelineDescriptor::RenderPipelineDescriptor(_)
            | PipelineDescriptor::ComputePipelineDescriptor(_) => unreachable!(),
            PipelineDescriptor::MeshPipelineDescriptor(descriptor) => descriptor,
        }
    }

    /// Try to retrieve a render pipeline GPU object from a cached ID.
    ///
    /// # Returns
    ///
    /// This method returns a successfully created render pipeline if any, or `None` if the pipeline
    /// was not created yet or if there was an error during creation. You can check the actual creation
    /// state with [`PipelineCache::get_render_pipeline_state()`].
    #[inline]
    pub fn get_render_pipeline(&self, id: CachedRenderPipelineId) -> Option<&RenderPipeline> {
        if let CachedPipelineState::Ok(
            Pipeline::RenderPipeline(pipeline) | Pipeline::MeshPipeline(pipeline),
        ) = &self.pipelines.get(id.id())?.state
        {
            Some(pipeline)
        } else {
            None
        }
    }

    /// opentennis P47: sets [`Self::compile_budget`]'s time for one queue pass of on-thread compiles.
    pub fn set_compile_budget(&mut self, budget: Option<core::time::Duration>) {
        self.compile_budget = budget;
    }

    /// Whether pipelines compile on the thread that processes the queue (no async compile task).
    fn compiles_in_place(&self) -> bool {
        cfg!(any(target_arch = "wasm32", target_os = "macos", not(feature = "multi_threaded"))) || self.synchronous_pipeline_compilation
    }

    /// Wait for a render pipeline to finish compiling.
    #[inline]
    ///
    /// opentennis P146: only this pipeline is compiled, here on the calling thread, ahead of the queue. Bevy's own
    /// version ran a whole queue pass first, which spawned every other queued pipeline onto the async pool ahead of
    /// this one, then waited for its turn: the compiles take the shader cache one at a time, so the render thread
    /// sat through all of them (iOS: 440-560 ms frames when a match or menu loaded with a dozen new pipelines).
    pub fn block_on_render_pipeline(&mut self, id: CachedRenderPipelineId) {
        if self.pipelines.len() <= id.id() {
            self.take_new_pipelines();
        }

        // still waiting on the pool behind other compiles: dropping the task cancels it if it hasn't started, and
        // the shader modules it may have made stay in the shader cache for the compile below
        let pending = match &mut self.pipelines[id.id()].state {
            CachedPipelineState::Creating(task) => bevy_tasks::futures::check_ready(task),
            _ => None,
        };
        if let Some(done) = pending {
            self.pipelines[id.id()].state = match done {
                Ok(p) => CachedPipelineState::Ok(p),
                Err(e) => CachedPipelineState::Err(e),
            };
        } else if matches!(self.pipelines[id.id()].state, CachedPipelineState::Creating(_)) {
            self.pipelines[id.id()].state = CachedPipelineState::Queued;
        }

        if matches!(self.pipelines[id.id()].state, CachedPipelineState::Queued) {
            if let PipelineDescriptor::RenderPipelineDescriptor(descriptor) = &self.pipelines[id.id()].descriptor {
                let descriptor = *descriptor.clone();
                let sync = mem::replace(&mut self.synchronous_pipeline_compilation, true);
                self.pipelines[id.id()].state = self.start_create_render_pipeline(id.id(), descriptor);
                self.synchronous_pipeline_compilation = sync;
            }
        }

        let state = &mut self.pipelines[id.id()].state;
        if let CachedPipelineState::Creating(task) = state {
            *state = match bevy_tasks::block_on(task) {
                Ok(p) => CachedPipelineState::Ok(p),
                Err(e) => CachedPipelineState::Err(e),
            };
        }
    }

    /// Try to retrieve a compute pipeline GPU object from a cached ID.
    ///
    /// # Returns
    ///
    /// This method returns a successfully created compute pipeline if any, or `None` if the pipeline
    /// was not created yet or if there was an error during creation. You can check the actual creation
    /// state with [`PipelineCache::get_compute_pipeline_state()`].
    #[inline]
    pub fn get_compute_pipeline(&self, id: CachedComputePipelineId) -> Option<&ComputePipeline> {
        if let CachedPipelineState::Ok(Pipeline::ComputePipeline(pipeline)) =
            &self.pipelines.get(id.id())?.state
        {
            Some(pipeline)
        } else {
            None
        }
    }

    /// Insert a render pipeline into the cache, and queue its creation.
    ///
    /// The pipeline is always inserted and queued for creation. There is no attempt to deduplicate it with
    /// an already cached pipeline.
    ///
    /// # Returns
    ///
    /// This method returns the unique render shader ID of the cached pipeline, which can be used to query
    /// the caching state with [`get_render_pipeline_state()`] and to retrieve the created GPU pipeline once
    /// it's ready with [`get_render_pipeline()`].
    ///
    /// [`get_render_pipeline_state()`]: PipelineCache::get_render_pipeline_state
    /// [`get_render_pipeline()`]: PipelineCache::get_render_pipeline
    pub fn queue_render_pipeline(
        &self,
        descriptor: RenderPipelineDescriptor,
    ) -> CachedRenderPipelineId {
        self.queue_render_pipeline_deduped(descriptor, true)
    }

    /// opentennis P55: queues a render pipeline nothing asked for yet, to have it compiled before it's needed (one
    /// recorded in an earlier run). It doesn't count as [`Self::requested`] until [`Self::queue_render_pipeline`] is
    /// called with an equal descriptor, which then gets this one's id.
    pub fn prewarm_render_pipeline(
        &self,
        descriptor: RenderPipelineDescriptor,
    ) -> CachedRenderPipelineId {
        self.queue_render_pipeline_deduped(descriptor, false)
    }

    /// opentennis P55: whether a pipeline was asked for by [`Self::queue_render_pipeline`] (or a compute one), not
    /// only pre-warmed.
    pub fn requested(&self, id: CachedPipelineId) -> bool {
        !self
            .dedup
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .unrequested
            .contains(&id)
    }

    /// opentennis P55: an equal render pipeline already in the cache, or a new one.
    fn queue_render_pipeline_deduped(
        &self,
        descriptor: RenderPipelineDescriptor,
        requested: bool,
    ) -> CachedRenderPipelineId {
        let mut new_pipelines = self
            .new_pipelines
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let mut dedup = self.dedup.lock().unwrap_or_else(PoisonError::into_inner);
        let hash = descriptor_hash(&descriptor);
        let len = self.pipelines.len();
        let same = dedup.by_hash.get(&hash).and_then(|ids| {
            ids.iter().copied().find(|&id| {
                let cached = if id < len { &self.pipelines[id] } else { &new_pipelines[id - len] };
                matches!(&cached.descriptor, PipelineDescriptor::RenderPipelineDescriptor(d) if **d == descriptor)
            })
        });
        if let Some(id) = same {
            if requested {
                dedup.unrequested.remove(&id);
            }
            return CachedRenderPipelineId::new(id);
        }
        let id = len + new_pipelines.len();
        dedup.by_hash.entry(hash).or_default().push(id);
        if !requested {
            dedup.unrequested.insert(id);
        }
        new_pipelines.push(CachedPipeline {
            descriptor: PipelineDescriptor::RenderPipelineDescriptor(Box::new(descriptor)),
            state: CachedPipelineState::Queued,
        });
        CachedRenderPipelineId::new(id)
    }

    /// Insert a compute pipeline into the cache, and queue its creation.
    ///
    /// The pipeline is always inserted and queued for creation. There is no attempt to deduplicate it with
    /// an already cached pipeline.
    ///
    /// # Returns
    ///
    /// This method returns the unique compute shader ID of the cached pipeline, which can be used to query
    /// the caching state with [`get_compute_pipeline_state()`] and to retrieve the created GPU pipeline once
    /// it's ready with [`get_compute_pipeline()`].
    ///
    /// [`get_compute_pipeline_state()`]: PipelineCache::get_compute_pipeline_state
    /// [`get_compute_pipeline()`]: PipelineCache::get_compute_pipeline
    pub fn queue_compute_pipeline(
        &self,
        descriptor: ComputePipelineDescriptor,
    ) -> CachedComputePipelineId {
        let mut new_pipelines = self
            .new_pipelines
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let id = CachedComputePipelineId::new(self.pipelines.len() + new_pipelines.len());
        new_pipelines.push(CachedPipeline {
            descriptor: PipelineDescriptor::ComputePipelineDescriptor(Box::new(descriptor)),
            state: CachedPipelineState::Queued,
        });
        id
    }

    /// Insert a mesh pipeline into the cache, and queue its creation.
    ///
    /// The pipeline is always inserted and queued for creation. There is no attempt to deduplicate it with
    /// an already cached pipeline.
    ///
    /// # Returns
    ///
    /// This method returns the unique mesh shader ID of the cached pipeline, which can be used to query
    /// the caching state with [`get_render_pipeline_state()`] and to retrieve the created GPU pipeline once
    /// it's ready with [`get_render_pipeline()`].
    ///
    /// [`get_render_pipeline_state()`]: PipelineCache::get_render_pipeline_state
    /// [`get_render_pipeline()`]: PipelineCache::get_render_pipeline
    pub fn queue_mesh_pipeline(
        &self,
        descriptor: MeshPipelineDescriptor,
    ) -> CachedRenderPipelineId {
        let mut new_pipelines = self
            .new_pipelines
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let id = CachedRenderPipelineId::new(self.pipelines.len() + new_pipelines.len());
        new_pipelines.push(CachedPipeline {
            descriptor: PipelineDescriptor::MeshPipelineDescriptor(Box::new(descriptor)),
            state: CachedPipelineState::Queued,
        });
        id
    }

    pub fn get_bind_group_layout(
        &self,
        bind_group_layout_descriptor: &BindGroupLayoutDescriptor,
    ) -> BindGroupLayout {
        self.bindgroup_layout_cache
            .lock()
            .unwrap()
            .get(&self.device, bind_group_layout_descriptor)
    }

    /// Inserts a [`Shader`] into this cache with the provided [`AssetId`].
    pub fn set_shader(&mut self, id: AssetId<Shader>, shader: Shader) {
        let mut shader_cache = self.shader_cache.lock().unwrap();
        let pipelines_to_queue = shader_cache.set_shader(id, shader);
        for cached_pipeline in pipelines_to_queue {
            self.pipelines[cached_pipeline].state = CachedPipelineState::Queued;
            self.waiting_pipelines.insert(cached_pipeline);
        }
    }

    /// Removes a [`Shader`] from this cache if it exists.
    pub fn remove_shader(&mut self, shader: AssetId<Shader>) {
        let mut shader_cache = self.shader_cache.lock().unwrap();
        let pipelines_to_queue = shader_cache.remove(shader);
        for cached_pipeline in pipelines_to_queue {
            self.pipelines[cached_pipeline].state = CachedPipelineState::Queued;
            self.waiting_pipelines.insert(cached_pipeline);
        }
    }

    fn start_create_render_pipeline(
        &mut self,
        id: CachedPipelineId,
        descriptor: RenderPipelineDescriptor,
    ) -> CachedPipelineState {
        let device = self.device.clone();
        let shader_cache = self.shader_cache.clone();
        let layout_cache = self.layout_cache.clone();
        let mut bindgroup_layout_cache = self.bindgroup_layout_cache.lock().unwrap();
        let bind_group_layout = descriptor
            .layout
            .iter()
            .map(|bind_group_layout_descriptor| {
                bindgroup_layout_cache.get(&self.device, bind_group_layout_descriptor)
            })
            .collect::<SmallVec<[_; BIND_GROUP_LAYOUTS_INLINE_CAPACITY]>>();

        create_pipeline_task(
            async move {
                let mut shader_cache = shader_cache.lock().unwrap();
                let mut layout_cache = layout_cache.lock().unwrap();

                let vertex_module = match shader_cache.get(
                    id,
                    descriptor.vertex.shader.id(),
                    &descriptor.vertex.shader_defs,
                ) {
                    Ok(module) => module,
                    Err(err) => return Err(err),
                };

                let fragment_module = match &descriptor.fragment {
                    Some(fragment) => {
                        match shader_cache.get(id, fragment.shader.id(), &fragment.shader_defs) {
                            Ok(module) => Some(module),
                            Err(err) => return Err(err),
                        }
                    }
                    None => None,
                };

                let layout = if descriptor.layout.is_empty() && descriptor.immediate_size == 0 {
                    None
                } else {
                    Some(layout_cache.get(&device, &bind_group_layout, descriptor.immediate_size))
                };

                drop((shader_cache, layout_cache));

                let vertex_buffer_layouts = descriptor
                    .vertex
                    .buffers
                    .iter()
                    .map(|layout| {
                        Some(RawVertexBufferLayout {
                            array_stride: layout.array_stride,
                            attributes: &layout.attributes,
                            step_mode: layout.step_mode,
                        })
                    })
                    .collect::<Vec<_>>();

                let fragment_data = descriptor.fragment.as_ref().map(|fragment| {
                    (
                        fragment_module.unwrap(),
                        fragment.entry_point.as_deref(),
                        fragment.targets.as_slice(),
                        fragment
                            .constants
                            .iter()
                            .map(|(k, v)| (k.as_ref(), *v))
                            .collect::<Vec<_>>(),
                    )
                });

                let vertex_constants: Vec<(&str, f64)> = descriptor
                    .vertex
                    .constants
                    .iter()
                    .map(|(k, v)| (k.as_ref(), *v))
                    .collect();

                let descriptor = RawRenderPipelineDescriptor {
                    multiview_mask: None,
                    depth_stencil: descriptor.depth_stencil.clone(),
                    label: descriptor.label.as_deref(),
                    layout: layout.as_ref().map(|layout| -> &PipelineLayout { layout }),
                    multisample: descriptor.multisample,
                    primitive: descriptor.primitive,
                    vertex: RawVertexState {
                        buffers: &vertex_buffer_layouts,
                        entry_point: descriptor.vertex.entry_point.as_deref(),
                        module: &vertex_module,
                        compilation_options: PipelineCompilationOptions {
                            constants: &vertex_constants,
                            zero_initialize_workgroup_memory: descriptor
                                .zero_initialize_workgroup_memory,
                        },
                    },
                    fragment: fragment_data.as_ref().map(
                        |(module, entry_point, targets, constants)| RawFragmentState {
                            entry_point: entry_point.as_deref(),
                            module,
                            targets,
                            compilation_options: PipelineCompilationOptions {
                                constants,
                                zero_initialize_workgroup_memory: descriptor
                                    .zero_initialize_workgroup_memory,
                            },
                        },
                    ),
                    cache: None,
                };

                Ok(Pipeline::RenderPipeline(
                    device.create_render_pipeline(&descriptor),
                ))
            },
            self.synchronous_pipeline_compilation,
        )
    }

    fn start_create_compute_pipeline(
        &mut self,
        id: CachedPipelineId,
        descriptor: ComputePipelineDescriptor,
    ) -> CachedPipelineState {
        let device = self.device.clone();
        let shader_cache = self.shader_cache.clone();
        let layout_cache = self.layout_cache.clone();
        let mut bindgroup_layout_cache = self.bindgroup_layout_cache.lock().unwrap();
        let bind_group_layout = descriptor
            .layout
            .iter()
            .map(|bind_group_layout_descriptor| {
                bindgroup_layout_cache.get(&self.device, bind_group_layout_descriptor)
            })
            .collect::<SmallVec<[_; BIND_GROUP_LAYOUTS_INLINE_CAPACITY]>>();

        create_pipeline_task(
            async move {
                let mut shader_cache = shader_cache.lock().unwrap();
                let mut layout_cache = layout_cache.lock().unwrap();

                let compute_module =
                    match shader_cache.get(id, descriptor.shader.id(), &descriptor.shader_defs) {
                        Ok(module) => module,
                        Err(err) => return Err(err),
                    };

                let layout = if descriptor.layout.is_empty() && descriptor.immediate_size == 0 {
                    None
                } else {
                    Some(layout_cache.get(&device, &bind_group_layout, descriptor.immediate_size))
                };

                drop((shader_cache, layout_cache));

                let constants: Vec<(&str, f64)> = descriptor
                    .constants
                    .iter()
                    .map(|(k, v)| (k.as_ref(), *v))
                    .collect();
                let descriptor = RawComputePipelineDescriptor {
                    label: descriptor.label.as_deref(),
                    layout: layout.as_ref().map(|layout| -> &PipelineLayout { layout }),
                    module: &compute_module,
                    entry_point: descriptor.entry_point.as_deref(),
                    compilation_options: PipelineCompilationOptions {
                        constants: &constants,
                        zero_initialize_workgroup_memory: descriptor
                            .zero_initialize_workgroup_memory,
                    },
                    cache: None,
                };

                Ok(Pipeline::ComputePipeline(
                    device.create_compute_pipeline(&descriptor),
                ))
            },
            self.synchronous_pipeline_compilation,
        )
    }

    fn start_create_mesh_pipeline(
        &mut self,
        id: CachedPipelineId,
        descriptor: MeshPipelineDescriptor,
    ) -> CachedPipelineState {
        let device = self.device.clone();
        let shader_cache = self.shader_cache.clone();
        let layout_cache = self.layout_cache.clone();
        let mut bindgroup_layout_cache = self.bindgroup_layout_cache.lock().unwrap();
        let bind_group_layout = descriptor
            .layout
            .iter()
            .map(|bind_group_layout_descriptor| {
                bindgroup_layout_cache.get(&self.device, bind_group_layout_descriptor)
            })
            .collect::<SmallVec<[_; BIND_GROUP_LAYOUTS_INLINE_CAPACITY]>>();

        create_pipeline_task(
            async move {
                let mut shader_cache = shader_cache.lock().unwrap();
                let mut layout_cache = layout_cache.lock().unwrap();

                let mesh_module = match shader_cache.get(
                    id,
                    descriptor.mesh.shader.id(),
                    &descriptor.mesh.shader_defs,
                ) {
                    Ok(module) => module,
                    Err(err) => return Err(err),
                };

                let fragment_module = match &descriptor.fragment {
                    Some(fragment) => {
                        match shader_cache.get(id, fragment.shader.id(), &fragment.shader_defs) {
                            Ok(module) => Some(module),
                            Err(err) => return Err(err),
                        }
                    }
                    None => None,
                };

                let task_module = match &descriptor.task {
                    Some(task) => match shader_cache.get(id, task.shader.id(), &task.shader_defs) {
                        Ok(module) => Some(module),
                        Err(err) => return Err(err),
                    },
                    None => None,
                };

                let layout = if descriptor.layout.is_empty() && descriptor.immediate_size == 0 {
                    None
                } else {
                    Some(layout_cache.get(&device, &bind_group_layout, descriptor.immediate_size))
                };

                drop((shader_cache, layout_cache));

                let fragment_data = descriptor.fragment.as_ref().map(|fragment| {
                    (
                        fragment_module.unwrap(),
                        fragment.entry_point.as_deref(),
                        fragment.targets.as_slice(),
                        fragment
                            .constants
                            .iter()
                            .map(|(k, v)| (k.as_ref(), *v))
                            .collect::<Vec<_>>(),
                    )
                });

                let mesh_constants: Vec<(&str, f64)> = descriptor
                    .mesh
                    .constants
                    .iter()
                    .map(|(k, v)| (k.as_ref(), *v))
                    .collect();

                let task_data = descriptor.task.as_ref().map(|task| {
                    (
                        task_module.unwrap(),
                        task.entry_point.as_deref(),
                        task.constants
                            .iter()
                            .map(|(k, v)| (k.as_ref(), *v))
                            .collect::<Vec<_>>(),
                    )
                });

                let descriptor = RawMeshPipelineDescriptor {
                    multiview: None,
                    depth_stencil: descriptor.depth_stencil.clone(),
                    label: descriptor.label.as_deref(),
                    layout: layout.as_ref().map(|layout| -> &PipelineLayout { layout }),
                    multisample: descriptor.multisample,
                    primitive: descriptor.primitive,
                    task: task_data
                        .as_ref()
                        .map(|(module, entry_point, constants)| RawTaskState {
                            entry_point: entry_point.as_deref(),
                            module,
                            compilation_options: PipelineCompilationOptions {
                                constants,
                                zero_initialize_workgroup_memory: descriptor
                                    .zero_initialize_workgroup_memory,
                            },
                        }),
                    mesh: RawMeshState {
                        module: &mesh_module,
                        entry_point: descriptor.mesh.entry_point.as_deref(),
                        compilation_options: PipelineCompilationOptions {
                            constants: &mesh_constants,
                            zero_initialize_workgroup_memory: descriptor
                                .zero_initialize_workgroup_memory,
                        },
                    },
                    fragment: fragment_data.as_ref().map(
                        |(module, entry_point, targets, constants)| RawFragmentState {
                            entry_point: entry_point.as_deref(),
                            module,
                            targets,
                            compilation_options: PipelineCompilationOptions {
                                constants,
                                zero_initialize_workgroup_memory: descriptor
                                    .zero_initialize_workgroup_memory,
                            },
                        },
                    ),
                    cache: None,
                };

                Ok(Pipeline::MeshPipeline(
                    device.create_mesh_pipeline(&descriptor),
                ))
            },
            self.synchronous_pipeline_compilation,
        )
    }
    /// opentennis P146: moves the pipelines queued since the last pass into the cache, waiting, without starting any.
    fn take_new_pipelines(&mut self) {
        let mut new_pipelines = self
            .new_pipelines
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        for new_pipeline in new_pipelines.drain(..) {
            self.waiting_pipelines.insert(self.pipelines.len());
            self.pipelines.push(new_pipeline);
        }
    }

    /// Process the pipeline queue and create all pending pipelines if possible.
    ///
    /// This is generally called automatically during the [`RenderSystems::Render`] step, but can
    /// be called manually to force creation at a different time.
    ///
    /// [`RenderSystems::Render`]: crate::RenderSystems::Render
    pub fn process_queue(&mut self) {
        let mut waiting_pipelines = mem::take(&mut self.waiting_pipelines);
        let mut pipelines = mem::take(&mut self.pipelines);

        {
            let mut new_pipelines = self
                .new_pipelines
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            for new_pipeline in new_pipelines.drain(..) {
                let id = pipelines.len();
                pipelines.push(new_pipeline);
                waiting_pipelines.insert(id);
            }
        }

        // opentennis P47: on-thread compiles stop starting once the pass's budget is spent (see `compile_budget`)
        let budget = self.compile_budget.filter(|_| self.compiles_in_place());
        let start = bevy_platform::time::Instant::now();
        let mut started = false;
        for id in waiting_pipelines {
            if let Some(budget) = budget
                && started
                && matches!(pipelines[id].state, CachedPipelineState::Queued)
                && start.elapsed() >= budget
            {
                self.waiting_pipelines.insert(id);
                continue;
            }
            started |= matches!(pipelines[id].state, CachedPipelineState::Queued);
            self.process_pipeline(&mut pipelines[id], id);
        }

        self.pipelines = pipelines;
    }

    fn process_pipeline(&mut self, cached_pipeline: &mut CachedPipeline, id: usize) {
        match &mut cached_pipeline.state {
            CachedPipelineState::Queued => {
                cached_pipeline.state = match &cached_pipeline.descriptor {
                    PipelineDescriptor::RenderPipelineDescriptor(descriptor) => {
                        self.start_create_render_pipeline(id, *descriptor.clone())
                    }
                    PipelineDescriptor::ComputePipelineDescriptor(descriptor) => {
                        self.start_create_compute_pipeline(id, *descriptor.clone())
                    }
                    PipelineDescriptor::MeshPipelineDescriptor(descriptor) => {
                        self.start_create_mesh_pipeline(id, *descriptor.clone())
                    }
                };
            }

            CachedPipelineState::Creating(task) => match bevy_tasks::futures::check_ready(task) {
                Some(Ok(pipeline)) => {
                    cached_pipeline.state = CachedPipelineState::Ok(pipeline);
                    return;
                }
                Some(Err(err)) => cached_pipeline.state = CachedPipelineState::Err(err),
                _ => (),
            },

            CachedPipelineState::Err(err) => match err {
                // Retry
                ShaderCacheError::ShaderNotLoaded(_)
                | ShaderCacheError::ShaderImportNotYetAvailable => {
                    bevy_log::debug!("retry processing pipeline {id}: {err}");
                    cached_pipeline.state = CachedPipelineState::Queued;
                }

                // Shader could not be processed ... retrying won't help
                ShaderCacheError::ProcessShaderError(error_detail) => {
                    let error_detail = error_detail.clone();
                    if std::env::var("VERBOSE_SHADER_ERROR")
                        .is_ok_and(|v| !(v.is_empty() || v == "0" || v == "false"))
                    {
                        error!("{}", pipeline_error_context(cached_pipeline));
                    }
                    error!("failed to process shader error:\n{}", error_detail);
                    return;
                }
                ShaderCacheError::CreateShaderModule(description) => {
                    error!("failed to create shader module: {}", description);
                    return;
                }
            },

            CachedPipelineState::Ok(_) => return,
        }

        // Retry
        self.waiting_pipelines.insert(id);
    }

    pub(crate) fn process_pipeline_queue_system(mut cache: ResMut<Self>) {
        cache.process_queue();
    }

    pub(crate) fn extract_shaders(
        mut cache: ResMut<Self>,
        shaders: Extract<Res<Assets<Shader>>>,
        mut events: Extract<MessageReader<AssetEvent<Shader>>>,
    ) {
        if cache.needs_shader_reload {
            cache.needs_shader_reload = false;
            for (id, shader) in shaders.iter() {
                let mut shader = shader.clone();
                shader
                    .shader_defs
                    .splice(0..0, cache.global_shader_defs.iter().cloned());
                cache.set_shader(id, shader);
            }
            // Drain events so we don't double-process shaders we just loaded.
            for _ in events.read() {}
            return;
        }

        for event in events.read() {
            #[expect(
                clippy::match_same_arms,
                reason = "LoadedWithDependencies is marked as a TODO, so it's likely this will no longer lint soon."
            )]
            match event {
                // PERF: Instead of blocking waiting for the shader cache lock, try again next frame if the lock is currently held
                AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                    if let Some(shader) = shaders.get(*id) {
                        let mut shader = shader.clone();
                        shader
                            .shader_defs
                            .splice(0..0, cache.global_shader_defs.iter().cloned());

                        cache.set_shader(*id, shader);
                    }
                }
                AssetEvent::Removed { id } => cache.remove_shader(*id),
                AssetEvent::Unused { .. } => {}
                AssetEvent::LoadedWithDependencies { .. } => {
                    // TODO: handle this
                }
            }
        }
    }
}

fn pipeline_error_context(cached_pipeline: &CachedPipeline) -> String {
    fn format(
        shader: &Handle<Shader>,
        entry: &Option<Cow<'static, str>>,
        shader_defs: &[ShaderDefVal],
    ) -> String {
        let source = match shader.path() {
            Some(path) => path.path().to_string_lossy().to_string(),
            None => String::new(),
        };
        let entry = match entry {
            Some(entry) => entry.to_string(),
            None => String::new(),
        };
        let shader_defs = shader_defs
            .iter()
            .flat_map(|def| match def {
                ShaderDefVal::Bool(k, v) if *v => Some(k.to_string()),
                ShaderDefVal::Int(k, v) => Some(format!("{k} = {v}")),
                ShaderDefVal::UInt(k, v) => Some(format!("{k} = {v}")),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("{source}:{entry}\nshader defs: {shader_defs}")
    }
    match &cached_pipeline.descriptor {
        PipelineDescriptor::RenderPipelineDescriptor(desc) => {
            let vert = &desc.vertex;
            let vert_str = format(&vert.shader, &vert.entry_point, &vert.shader_defs);
            let Some(frag) = desc.fragment.as_ref() else {
                return vert_str;
            };
            let frag_str = format(&frag.shader, &frag.entry_point, &frag.shader_defs);
            format!("vertex {vert_str}\nfragment {frag_str}")
        }
        PipelineDescriptor::ComputePipelineDescriptor(desc) => {
            format(&desc.shader, &desc.entry_point, &desc.shader_defs)
        }
        PipelineDescriptor::MeshPipelineDescriptor(desc) => {
            let mesh = &desc.mesh;
            let mesh_str = format(&mesh.shader, &mesh.entry_point, &mesh.shader_defs);

            let task = &desc.task;
            let task_str = task
                .as_ref()
                .map(|task| format(&task.shader, &task.entry_point, &task.shader_defs));

            let frag = &desc.fragment;
            let frag_str = frag
                .as_ref()
                .map(|frag| format(&frag.shader, &frag.entry_point, &frag.shader_defs));

            match (task_str, frag_str) {
                (None, None) => format!("mesh {mesh_str}"),
                (None, Some(frag_str)) => format!("mesh {mesh_str}\nfragment {frag_str}"),
                (Some(task_str), None) => format!("task {task_str}\nmesh {mesh_str}"),
                (Some(task_str), Some(frag_str)) => {
                    format!("task {task_str}\nmesh {mesh_str}\nfragment {frag_str}")
                }
            }
        }
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "macos"),
    feature = "multi_threaded"
))]
fn create_pipeline_task(
    task: impl Future<Output = Result<Pipeline, ShaderCacheError>> + Send + 'static,
    sync: bool,
) -> CachedPipelineState {
    if !sync {
        return CachedPipelineState::Creating(bevy_tasks::AsyncComputeTaskPool::get().spawn(task));
    }

    match bevy_tasks::block_on(task) {
        Ok(pipeline) => CachedPipelineState::Ok(pipeline),
        Err(err) => CachedPipelineState::Err(err),
    }
}

#[cfg(any(
    target_arch = "wasm32",
    target_os = "macos",
    not(feature = "multi_threaded")
))]
fn create_pipeline_task(
    task: impl Future<Output = Result<Pipeline, ShaderCacheError>> + Send + 'static,
    _sync: bool,
) -> CachedPipelineState {
    match bevy_tasks::block_on(task) {
        Ok(pipeline) => CachedPipelineState::Ok(pipeline),
        Err(err) => CachedPipelineState::Err(err),
    }
}
