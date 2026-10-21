# P136 — pipeline pre-warm record survives shader binding changes

B629 bumped `pipelines.rs` VERSION by hand because a recorded pipeline whose shader's bindings had moved was
pre-warmed with its stale bind-group layout, failed wgpu validation and quit the app.

## What changed (`crates/hst-base/src/pipelines.rs`)
- Each recorded stage stores a hash of its shader: FNV-1a 64 (stable across runs/platforms, not std's hasher) over the
  shader's source, its own shader defs, and the same for every module it imports, transitively (imports resolved by
  import path among the loaded shaders; `additional_imports` too). Kept per shader for the run (`Hashes`).
- `Warmup::feed` compares it with this build's shader: equal → queued; different → left out (`Changed`, counted in the
  log line as "changed since"); shader or an import still loading (asset-server `embedded://` shaders) → retried on a
  later feed call, given up after 10 s (`WAIT_MAX`) or when the load failed.
- A pipeline is only recorded once its shader and all imports are loaded (it compiled, so they are).
- VERSION 3 (the new field): existing records are dropped once more, and no shader edit needs a bump again.
- Why the hash and not "drop entries that fail validation": wgpu validation errors come back through error scopes,
  which are asynchronous on the web, so a failing pipeline can't be caught there before it takes the app down; the
  hash check keeps the stale descriptor from ever reaching wgpu on every platform.

## Tests
`pipelines::tests::a_pipeline_whose_shader_changed_is_left_out`: a record made with one build feeds back with the same
shaders, is left out when the shader or an imported module's bindings change, waits while an import isn't loaded.
`a_recorded_pipeline_comes_back_equal` still passes.

## Not verified / not 1:1
- Not tried live (no real stale record replayed in the app): covered by the unit test only. Not gameplay, nothing to
  match against the original.
- A pipeline whose bind-group layout changes in Rust while its shader source and defs stay identical isn't caught; such
  a pipeline would also fail for the game's own request, so it can't ship.
- Hot-reloading a shader within one run keeps its first hash (dev-only).
