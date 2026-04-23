// Body 3D rendering shader.
//
// Three pipelines share this WGSL:
//   * vs_face / fs_face   — per-pixel Phong shading on triangulated bodies
//   * vs_edge / fs_edge   — screen-space expanded, feathered edge quads
//   * vs_blit / fs_blit   — fullscreen-triangle copy from offscreen target
//                           into the egui color attachment

struct Camera {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    viewport_size_px: vec4<f32>,
    edge_params: vec4<f32>, // x=half_width_px y=feather_px
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
    @location(0) start: vec3<f32>,
    @location(1) end: vec3<f32>,
    @location(2) color: vec4<f32>,
};

struct EdgeVsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) side_px: f32,
    @location(2) along_px: f32,
    @location(3) length_px: f32,
};

fn pixel_delta_to_ndc(delta_px: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(
        (2.0 * delta_px.x) / camera.viewport_size_px.x,
        (-2.0 * delta_px.y) / camera.viewport_size_px.y,
    );
}

@vertex
fn vs_edge(@builtin(vertex_index) vertex_index: u32, in: EdgeVsIn) -> EdgeVsOut {
    var out: EdgeVsOut;

    var start_clip = camera.view_proj * vec4<f32>(in.start, 1.0);
    var end_clip = camera.view_proj * vec4<f32>(in.end, 1.0);
    // Pull edges very slightly toward the camera in clip space so they
    // win the depth fight against their own coplanar faces. The factor
    // is multiplied by `w` to stay perspective-correct.
    start_clip.z = start_clip.z - 0.00015 * start_clip.w;
    end_clip.z = end_clip.z - 0.00015 * end_clip.w;

    let start_ndc = start_clip.xy / start_clip.w;
    let end_ndc = end_clip.xy / end_clip.w;
    let segment_px = vec2<f32>(
        (end_ndc.x - start_ndc.x) * camera.viewport_size_px.x * 0.5,
        (start_ndc.y - end_ndc.y) * camera.viewport_size_px.y * 0.5,
    );
    let segment_len = length(segment_px);

    var tangent_px = vec2<f32>(1.0, 0.0);
    if segment_len > 1e-4 {
        tangent_px = segment_px / segment_len;
    }
    let normal_px = vec2<f32>(-tangent_px.y, tangent_px.x);

    let half_width = camera.edge_params.x;
    let feather = max(camera.edge_params.y, 0.001);
    let half_extent = half_width + feather;

    var use_end = false;
    var side_px = -half_extent;
    var along_px = -feather;
    switch (vertex_index % 6u) {
        case 0u, 3u: {
            use_end = false;
            side_px = -half_extent;
            along_px = -feather;
        }
        case 1u: {
            use_end = true;
            side_px = -half_extent;
            along_px = segment_len + feather;
        }
        case 2u, 4u: {
            use_end = true;
            side_px = half_extent;
            along_px = segment_len + feather;
        }
        default: {
            use_end = false;
            side_px = half_extent;
            along_px = -feather;
        }
    }

    let base_clip = select(start_clip, end_clip, use_end);
    let along_offset_px = select(-feather, feather, use_end);
    let ndc_offset = pixel_delta_to_ndc(tangent_px * along_offset_px + normal_px * side_px);
    out.clip = base_clip + vec4<f32>(ndc_offset * base_clip.w, 0.0, 0.0);
    out.color = in.color;
    out.side_px = side_px;
    out.along_px = along_px;
    out.length_px = segment_len;
    return out;
}

@fragment
fn fs_edge(in: EdgeVsOut) -> @location(0) vec4<f32> {
    let half_width = camera.edge_params.x;
    let feather = max(camera.edge_params.y, 0.001);
    let side_dist = abs(in.side_px) - half_width;
    let cap_dist = max(-in.along_px, in.along_px - in.length_px);
    let outside = max(side_dist, cap_dist);
    let coverage = 1.0 - smoothstep(0.0, feather, outside);
    return vec4<f32>(in.color.rgb, in.color.a * coverage);
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
