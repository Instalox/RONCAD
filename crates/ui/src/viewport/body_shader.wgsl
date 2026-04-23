// Body 3D rendering shader.
//
// Three pipelines share this WGSL:
//   * vs_face / fs_face   — per-pixel Phong shading on triangulated bodies
//   * vs_edge / fs_edge   — line-list rendering of classified edges
//   * vs_blit / fs_blit   — fullscreen-triangle copy from offscreen target
//                           into the egui color attachment

struct Camera {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    light_key_dir: vec4<f32>,
    light_key_color: vec4<f32>,
    light_fill_dir: vec4<f32>,
    light_fill_color: vec4<f32>,
    light_back_dir: vec4<f32>,
    light_back_color: vec4<f32>,
    ambient_sky: vec4<f32>,
    ambient_ground: vec4<f32>,
    spec_params: vec4<f32>, // x=spec_power y=spec_weight z=rim_power w=rim_weight
};

@group(0) @binding(0) var<uniform> camera: Camera;

// ----------------------------------------------------------------------------
// Faces
// ----------------------------------------------------------------------------

struct FaceVsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) base_color: vec4<f32>,
};

struct FaceVsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) albedo: vec3<f32>,
    @location(3) emissive: f32,
};

@vertex
fn vs_face(in: FaceVsIn) -> FaceVsOut {
    var out: FaceVsOut;
    out.clip = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.world_pos = in.position;
    out.normal = in.normal;
    out.albedo = in.base_color.rgb;
    out.emissive = in.base_color.a;
    return out;
}

@fragment
fn fs_face(in: FaceVsOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let v = normalize(camera.eye.xyz - in.world_pos);

    // Hemisphere ambient: lighter on top-facing normals, darker below.
    let hemi_t = clamp(n.z * 0.5 + 0.5, 0.0, 1.0);
    let ambient = camera.ambient_sky.rgb * hemi_t + camera.ambient_ground.rgb * (1.0 - hemi_t);

    let key_dir  = camera.light_key_dir.xyz;
    let fill_dir = camera.light_fill_dir.xyz;
    let back_dir = camera.light_back_dir.xyz;

    let diff_key  = max(dot(n, key_dir),  0.0);
    let diff_fill = max(dot(n, fill_dir), 0.0);
    let diff_back = max(dot(n, back_dir), 0.0) * 0.25;

    let diffuse = camera.light_key_color.rgb  * diff_key
                + camera.light_fill_color.rgb * diff_fill
                + camera.light_back_color.rgb * diff_back;

    // Blinn-Phong specular from the key light only.
    let halfway = normalize(key_dir + v);
    let spec = pow(max(dot(n, halfway), 0.0), camera.spec_params.x) * camera.spec_params.y;
    let specular = camera.light_key_color.rgb * spec;

    // Fresnel rim term picks out the silhouette.
    let fresnel = pow(clamp(1.0 - max(dot(n, v), 0.0), 0.0, 1.0), camera.spec_params.z);
    let rim = camera.light_back_color.rgb * fresnel * camera.spec_params.w;

    // Selected bodies pass an emissive lift in the alpha channel of the
    // vertex base color so they read as "lit from within" without being
    // washed out by the ambient term.
    let emissive = in.albedo * in.emissive;

    let lit = in.albedo * (ambient + diffuse) + specular + rim + emissive;
    return vec4<f32>(lit, 1.0);
}

// ----------------------------------------------------------------------------
// Edges
// ----------------------------------------------------------------------------

struct EdgeVsIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct EdgeVsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_edge(in: EdgeVsIn) -> EdgeVsOut {
    var out: EdgeVsOut;
    var clip = camera.view_proj * vec4<f32>(in.position, 1.0);
    // Pull edges very slightly toward the camera in clip space so they
    // win the depth fight against their own coplanar faces. The factor
    // is multiplied by `w` to stay perspective-correct.
    clip.z = clip.z - 0.00015 * clip.w;
    out.clip = clip;
    out.color = in.color;
    return out;
}

@fragment
fn fs_edge(in: EdgeVsOut) -> @location(0) vec4<f32> {
    return in.color;
}

// ----------------------------------------------------------------------------
// Blit (fullscreen triangle, source-over composite)
// ----------------------------------------------------------------------------

struct BlitVsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_blit(@builtin(vertex_index) idx: u32) -> BlitVsOut {
    // Three-vertex fullscreen triangle.
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    let p = corners[idx];
    var out: BlitVsOut;
    out.clip = vec4<f32>(p, 0.0, 1.0);
    // Flip y so we sample the offscreen with screen-down convention.
    out.uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    return out;
}

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;

@fragment
fn fs_blit(in: BlitVsOut) -> @location(0) vec4<f32> {
    return textureSample(src_tex, src_sampler, in.uv);
}
