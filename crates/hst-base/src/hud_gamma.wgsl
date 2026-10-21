// The window's last two passes (`hud_gamma/out.rs`), full-screen triangles: with PERSIST the frame persistence,
// else the composite.
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

#ifdef GLOW
// B629: the picture averaged down into the small glow image: each texel the mean of an 8×8 grid of bilinear taps
// spread over its share of the picture, so the bars' glow has no detail left to shimmer or streak.
@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var linear: sampler;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // one texel of the glow image (`out::GLOW_SIZE` square) in uv
    let texel = vec2(1.0 / 48.0);
    var sum = vec3(0.0);
    for (var j = 0; j < 8; j++) {
        for (var i = 0; i < 8; i++) {
            let at = in.uv + (vec2(f32(i), f32(j)) + 0.5 - 4.0) / 8.0 * texel;
            sum += textureSampleLevel(scene, linear, at, 0.0).rgb;
        }
    }
    return vec4(sum / 64.0, 1.0);
}
#else ifdef PERSIST
// the match's frame persistence: this frame's scene with `weight.x` of the last frame shown mixed in, on the
// encoded values as the GS blends (Cd + (Cs − Cd)·FIX, Cs the last frame, FIX 0x20 = 25 % at 60 Hz).
// Facing the sun (`weight.y` > 0) the sun glare instead: the frame drawn over itself twice at FIX `weight.y`,
// bilinear at (−½, −½) then (+½, +½) texels of the game's 640×224 field (`weight.zw` = one such texel in uv), so
// each pass mixes in the 2×2 average next to every pixel.
@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var last: texture_2d<f32>;
@group(0) @binding(2) var<uniform> weight: vec4<f32>;
@group(0) @binding(3) var linear: sampler;

fn f(uv: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(scene, linear, uv, 0.0).rgb;
}

// the first pass's result at uv: mixed with the average of its up-left 2×2
fn first(uv: vec2<f32>) -> vec3<f32> {
    let d = weight.zw;
    let avg = (f(uv) + f(uv - vec2(d.x, 0.0)) + f(uv - vec2(0.0, d.y)) + f(uv - d)) * 0.25;
    return mix(f(uv), avg, weight.y);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    if weight.y > 0.0 {
        let d = weight.zw;
        let p = first(in.uv);
        let avg = (p + first(in.uv + vec2(d.x, 0.0)) + first(in.uv + vec2(0.0, d.y)) + first(in.uv + d)) * 0.25;
        return vec4(mix(p, avg, weight.y), 1.0);
    }
    let p = vec2<i32>(in.position.xy);
    return vec4(mix(textureLoad(scene, p, 0).rgb, textureLoad(last, p, 0).rgb, weight.x), 1.0);
}
#else

// the composite: the HUD (premultiplied, linear) over the scene (gamma-encoded), blended on the encoded values as the
// GS does. B629: on the whole window, the scene in the picture's rect (`picture`); beside it (a fixed Aspect Ratio's
// bars) a blurred, dimmed glow of the picture's nearest edge, fading out away from it, at `glow.x` (0: black).
@group(0) @binding(0) var shown: texture_2d<f32>;
@group(0) @binding(1) var hud: texture_2d<f32>;
@group(0) @binding(2) var shown_sampler: sampler;
@group(0) @binding(3) var<uniform> frame: array<vec4<f32>, 2>;
@group(0) @binding(4) var glow_image: texture_2d<f32>;
@group(0) @binding(5) var glow_sampler: sampler;

fn enc(c: vec3<f32>) -> vec3<f32> {
    return select(1.055 * pow(c, vec3(1.0 / 2.4)) - 0.055, c * 12.92, c <= vec3(0.0031308));
}

fn dec(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3(2.4)), c / 12.92, c <= vec3(0.04045));
}

// the bars' colour at picture uv `p` (outside 0..1): the glow image (the picture averaged down) blurred again around
// the nearest edge, over a quarter of the picture's height, so its colours carry on into the bar softly
fn glow(p: vec2<f32>, aspect: f32) -> vec3<f32> {
    let q = clamp(p, vec2(0.0), vec2(1.0));
    let r = vec2(0.25 / aspect, 0.25) / 4.0;
    var sum = vec3(0.0);
    for (var j = -2; j <= 2; j++) {
        for (var i = -2; i <= 2; i++) {
            let at = clamp(q + vec2(f32(i), f32(j)) * r, vec2(0.0), vec2(1.0));
            sum += textureSampleLevel(glow_image, glow_sampler, at, 0.0).rgb;
        }
    }
    return sum / 25.0;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // the scene is drawn at the render scale setting: filtered up to the picture's rect (at 100 % this hits texel
    // centres); the HUD at the window's size, or at 448 lines of the picture in the original mode (nearest)
    let rect = frame[0];
    let p = (in.uv - rect.xy) / rect.zw;
    var b: vec3<f32>;
    if all(p >= vec2(0.0)) && all(p <= vec2(1.0)) {
        b = textureSampleLevel(shown, shown_sampler, p, 0.0).rgb;
    } else if frame[1].x > 0.0 {
        // how far out of the picture, in picture heights; the glow fades to black over half of one
        let wd = vec2<f32>(textureDimensions(hud));
        let aspect = rect.z * wd.x / max(rect.w * wd.y, 1.0);
        let d = (p - clamp(p, vec2(0.0), vec2(1.0))) * vec2(aspect, 1.0);
        let fade = 1.0 - smoothstep(0.0, 0.5, length(d));
        b = glow(p, aspect) * frame[1].x * fade;
    } else {
        b = vec3(0.0);
    }
    let hd = vec2<f32>(textureDimensions(hud));
    let h = textureLoad(hud, vec2<i32>(min(in.uv * hd, hd - 1.0)), 0);
    let c = select(vec3(0.0), h.rgb / h.a, h.a > 0.0);
    return vec4(dec(enc(c) * h.a + b * (1.0 - h.a)), 1.0);
}
#endif
