//! Raw Vulkan (ash). Knows nothing about icons, fonts or layout: it draws a [`Scene`] (flat and
//! textured 2D vertices, twice: the page and what goes over it) on a [`Target`] (a window of this
//! crate). What to draw is up to the app, see [`Content`].
// no HiDPI, 4x MSAA is the only antialiasing, unwrap() instead of error handling (a
// panic only kills the render thread, never the host).
#![allow(unsafe_op_in_unsafe_fn)] // the whole renderer is one unsafe block
use ash::{Entry, Instance, khr, vk};
use std::cell::Cell;
use std::ffi::CStr;
use std::io::Cursor;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};

use crate::images::{Images, SHEET};

use bx_shader::include_shader;

#[cfg(all(unix, not(target_os = "macos")))]
const EXTS: [&CStr; 2] = [khr::surface::NAME, khr::xlib_surface::NAME];
#[cfg(windows)]
const EXTS: [&CStr; 2] = [khr::surface::NAME, khr::win32_surface::NAME];

/// A native window that Vulkan can draw on: [`crate::window::Window`], [`crate::native::Native`].
///
/// # Safety
///
/// `surface` must return a valid surface for a window that stays alive as long as `self` is
/// borrowed, and nothing but the thread that calls it may touch the connection (X11 `Display`)
/// the surface was made from.
pub unsafe trait Target: Sync {
    /// Called on the render thread, once. The caller owns the surface and destroys it.
    fn surface(&self, entry: &Entry, inst: &Instance) -> vk::SurfaceKHR;
    /// Called on the render thread when the scene asks for another mouse cursor
    /// ([`Scene::cursor`]).
    fn set_cursor(&self, _cursor: u8) {}
}

/// The mouse cursors of [`Scene::cursor`].
pub const CURSOR_ARROW: u8 = 0;
pub const CURSOR_HAND: u8 = 1;
/// The I-beam over text.
pub const CURSOR_TEXT: u8 = 2;
/// The bottom right corner resize arrow.
pub const CURSOR_RESIZE: u8 = 3;

/// 4 samples is the one MSAA count the Vulkan spec requires every device to support.
const SAMPLES: vk::SampleCountFlags = vk::SampleCountFlags::TYPE_4;

/// Room for this many vertices (32 bytes each) in the vertex buffer: flat and textured together.
const MAX_VERTS: usize = 65536;

const SPV: &[u8] = include_shader!("shader.wgsl");

/// "The mouse is not on the window", see [`pack_mouse`].
pub const NO_MOUSE: u64 = u64::MAX;

/// Packs a mouse position (px from the top left of the window) for [`Content::mouse`].
pub const fn pack_mouse(x: i32, y: i32) -> u64 {
    (x as u32 as u64) << 32 | y as u32 as u64
}

/// Packs a window size for the `size` argument of [`render`].
pub const fn pack_size(w: u32, h: u32) -> u64 {
    (w as u64) << 32 | h as u64
}

/// The texture the `textured` vertices sample (R8G8B8A8_UNORM, i.e. no sRGB decoding).
pub struct Atlas {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

/// A 2D vertex: `[x, y, u, v, r, g, b, a]`, position in NDC with y down (see shader.wgsl).
pub type Vert = [f32; 8];

/// What to draw for one window size, as triangle lists.
pub struct Scene {
    /// Solid colour (the vertex colour); the uv is unused.
    pub flat: Vec<Vert>,
    /// Drawn after `flat`, sampled from the atlas (the MSDF fragment shader), tinted by the vertex colour.
    pub textured: Vec<Vert>,
    /// Like `flat` and `textured` again, drawn on top of both (a tooltip).
    pub over_flat: Vec<Vert>,
    pub over_textured: Vec<Vert>,
    /// Quads of the pixel images ([`Images`]): the uv is the cell, the colour tints the pixels
    /// (white: as they are). Drawn after `textured`, and `over_images` after `over_textured`.
    pub images: Vec<Vert>,
    pub over_images: Vec<Vert>,
    /// The mouse cursor: one of `CURSOR_*`.
    pub cursor: u8,
}

/// What the app brings to [`render`].
pub struct Content<'a> {
    /// For the `textured` vertices.
    pub atlas: &'a Atlas,
    /// The pixels of the `images` vertices; uploaded again whenever `Images::version` changes.
    pub images: &'a Mutex<Images>,
    /// Called (on the render thread) at the start, after every resize and whenever the mouse has
    /// moved, with the window size and the mouse in px from the top left (NaN when it is not on
    /// the window); it may return a different number of vertices each time, up to [`MAX_VERTS`]
    /// in total.
    pub scene: &'a dyn Fn(u32, u32, [f32; 2]) -> Scene,
    /// Where the mouse is, see [`pack_mouse`] (written by the window, polled every frame).
    pub mouse: &'a AtomicU64,
    /// Clicks and keys; the scene takes them (`Events::take`), a push makes the next frame draw
    /// the scene again.
    pub events: &'a crate::input::Events,
}

/// Everything that depends on the surface size; rebuilt when the window is resized.
struct Frame {
    sc: vk::SwapchainKHR,
    extent: vk::Extent2D,
    views: Vec<vk::ImageView>,
    fbs: Vec<vk::Framebuffer>,
    cmds: Vec<vk::CommandBuffer>,
    msaa: (vk::Image, vk::DeviceMemory, vk::ImageView),
}

fn color_range() -> vk::ImageSubresourceRange {
    vk::ImageSubresourceRange::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .level_count(1)
        .layer_count(1)
}

/// Creates a Vulkan instance, device and swapchain for `target`, then draws at 4x MSAA until
/// `stop` is set. The swapchain follows the size of the window, so the caller only has to resize
/// the window. Everything is destroyed again before this returns. Meant for a thread of its own.
///
/// `content` is what is drawn, see [`Content`]; its callbacks run on this thread.
///
/// `size` (see [`pack_size`]) is the current window size; it is only read where the surface does
/// not report its own size (Wayland), and must then be kept up to date by the caller. After every
/// presented frame its size (see [`pack_size`]) is stored in `presented`, so a window can wait for
/// a frame that fits before it lets go of a resize (no stale, stretched frame).
///
/// Each frame is acquire -> submit -> present -> `queue_wait_idle`. Frames are paced by a 16 ms
/// sleep while the size is steady, and by the presentation queue alone while it changes.
///
/// # Panics
///
/// On any Vulkan error (there is no error handling), without a Vulkan loader, if no device can
/// present to the window, or if a scene has too many vertices. Resources are leaked
/// then, which only matters if the process continues.
// queue_wait_idle instead of frames-in-flight, constant redraw instead of redraw on demand.
pub fn render(
    target: &impl Target,
    size: &AtomicU64,
    stop: &AtomicBool,
    presented: &AtomicU64,
    content: &Content,
) {
    // sound because of the contract of `Target`
    unsafe { run(target, size, stop, presented, content) }
}

unsafe fn run(
    target: &impl Target,
    size: &AtomicU64,
    stop: &AtomicBool,
    presented: &AtomicU64,
    content: &Content,
) {
    let (atlas, scene) = (content.atlas, content.scene);
    let entry = Entry::load().unwrap();
    let exts = EXTS.map(|e| e.as_ptr());
    let inst = entry
        .create_instance(
            &vk::InstanceCreateInfo::default().enabled_extension_names(&exts),
            None,
        )
        .unwrap();
    let surf_i = khr::surface::Instance::new(&entry, &inst);
    let surf = target.surface(&entry, &inst);

    let (pd, qf) = inst
        .enumerate_physical_devices()
        .unwrap()
        .into_iter()
        .find_map(|pd| {
            let fams = inst.get_physical_device_queue_family_properties(pd);
            (0..fams.len() as u32)
                .find(|&i| {
                    fams[i as usize]
                        .queue_flags
                        .contains(vk::QueueFlags::GRAPHICS)
                        && surf_i
                            .get_physical_device_surface_support(pd, i, surf)
                            .unwrap()
                })
                .map(|i| (pd, i))
        })
        .expect("no Vulkan device can present to this window");
    let dev = inst
        .create_device(
            pd,
            &vk::DeviceCreateInfo::default()
                .queue_create_infos(&[vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(qf)
                    .queue_priorities(&[1.0])])
                .enabled_extension_names(&[khr::swapchain::NAME.as_ptr()]),
            None,
        )
        .unwrap();
    let queue = dev.get_device_queue(qf, 0);
    let pool = dev
        .create_command_pool(
            &vk::CommandPoolCreateInfo::default().queue_family_index(qf),
            None,
        )
        .unwrap();

    let sc_d = khr::swapchain::Device::new(&inst, &dev);
    // UNORM, not SRGB: the colours are written as the sRGB values they are (`bx_ui::rgb`), an
    // SRGB format would encode them a second time and wash them out
    let fmts = surf_i
        .get_physical_device_surface_formats(pd, surf)
        .unwrap();
    let fmt = *fmts
        .iter()
        .find(|f| {
            matches!(
                f.format,
                vk::Format::B8G8R8A8_UNORM | vk::Format::R8G8B8A8_UNORM
            )
        })
        .unwrap_or(&fmts[0]);
    let mp = inst.get_physical_device_memory_properties(pd);
    let mem_type = |bits: u32, want: vk::MemoryPropertyFlags| {
        (0..mp.memory_type_count)
            .find(|&i| {
                bits & (1 << i) != 0 && mp.memory_types[i as usize].property_flags.contains(want)
            })
            .unwrap()
    };
    let host = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;

    // a buffer with its own memory
    let buffer = |bytes: usize, usage: vk::BufferUsageFlags| {
        let b = dev
            .create_buffer(
                &vk::BufferCreateInfo::default()
                    .size(bytes as u64)
                    .usage(usage)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE),
                None,
            )
            .unwrap();
        let req = dev.get_buffer_memory_requirements(b);
        let mem = dev
            .allocate_memory(
                &vk::MemoryAllocateInfo::default()
                    .allocation_size(req.size)
                    .memory_type_index(mem_type(req.memory_type_bits, host)),
                None,
            )
            .unwrap();
        dev.bind_buffer_memory(b, mem, 0).unwrap();
        (b, mem)
    };

    // vertex buffer, rewritten (while the GPU is idle) whenever the scene is rebuilt
    let (vb, vmem) = buffer(MAX_VERTS * 32, vk::BufferUsageFlags::VERTEX_BUFFER);
    let verts = dev
        .map_memory(vmem, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())
        .unwrap() as *mut Vert;

    // the atlas: staging buffer -> device-local image, left in SHADER_READ_ONLY_OPTIMAL
    let (stg, stg_mem) = buffer(atlas.rgba.len(), vk::BufferUsageFlags::TRANSFER_SRC);
    let p = dev
        .map_memory(stg_mem, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())
        .unwrap() as *mut u8;
    p.copy_from_nonoverlapping(atlas.rgba.as_ptr(), atlas.rgba.len());
    dev.unmap_memory(stg_mem);
    let extent3 = vk::Extent3D {
        width: atlas.w,
        height: atlas.h,
        depth: 1,
    };
    // a device-local RGBA image the shaders sample
    let texture = |e: vk::Extent3D| {
        let img = dev
            .create_image(
                &vk::ImageCreateInfo::default()
                    .image_type(vk::ImageType::TYPE_2D)
                    .format(vk::Format::R8G8B8A8_UNORM)
                    .extent(e)
                    .mip_levels(1)
                    .array_layers(1)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .tiling(vk::ImageTiling::OPTIMAL)
                    .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .initial_layout(vk::ImageLayout::UNDEFINED),
                None,
            )
            .unwrap();
        let req = dev.get_image_memory_requirements(img);
        let mem = dev
            .allocate_memory(
                &vk::MemoryAllocateInfo::default()
                    .allocation_size(req.size)
                    .memory_type_index(mem_type(
                        req.memory_type_bits,
                        vk::MemoryPropertyFlags::DEVICE_LOCAL,
                    )),
                None,
            )
            .unwrap();
        dev.bind_image_memory(img, mem, 0).unwrap();
        (img, mem)
    };
    // copies the whole of `stg` into `img` (what it held is dropped) and leaves it in
    // SHADER_READ_ONLY_OPTIMAL; waits for the GPU
    let upload = |img: vk::Image, stg: vk::Buffer, e: vk::Extent3D| {
        let cmd = dev
            .allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(pool)
                    .command_buffer_count(1),
            )
            .unwrap()[0];
        let barrier = |old, new, src_access, dst_access, src_stage, dst_stage| {
            dev.cmd_pipeline_barrier(
                cmd,
                src_stage,
                dst_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[vk::ImageMemoryBarrier::default()
                    .old_layout(old)
                    .new_layout(new)
                    .src_access_mask(src_access)
                    .dst_access_mask(dst_access)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .image(img)
                    .subresource_range(color_range())],
            )
        };
        dev.begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::default())
            .unwrap();
        barrier(
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::AccessFlags::empty(),
            vk::AccessFlags::TRANSFER_WRITE,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
        );
        dev.cmd_copy_buffer_to_image(
            cmd,
            stg,
            img,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[vk::BufferImageCopy::default()
                .image_subresource(
                    vk::ImageSubresourceLayers::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .layer_count(1),
                )
                .image_extent(e)],
        );
        barrier(
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::AccessFlags::TRANSFER_WRITE,
            vk::AccessFlags::SHADER_READ,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        );
        dev.end_command_buffer(cmd).unwrap();
        dev.queue_submit(
            queue,
            &[vk::SubmitInfo::default().command_buffers(&[cmd])],
            vk::Fence::null(),
        )
        .unwrap();
        dev.queue_wait_idle(queue).unwrap();
        dev.free_command_buffers(pool, &[cmd]);
    };
    let (tex, tmem) = texture(extent3);
    upload(tex, stg, extent3);
    dev.destroy_buffer(stg, None);
    dev.free_memory(stg_mem, None);

    let view = |img, format| {
        dev.create_image_view(
            &vk::ImageViewCreateInfo::default()
                .image(img)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .subresource_range(color_range()),
            None,
        )
        .unwrap()
    };
    let tview = view(tex, vk::Format::R8G8B8A8_UNORM);
    // the image sheet: rewritten from `content.images` (staging buffer, kept mapped) whenever its
    // version moved, see `record`
    let sheet_bytes = (SHEET * SHEET * 4) as usize;
    let sheet_ext = vk::Extent3D {
        width: SHEET,
        height: SHEET,
        depth: 1,
    };
    let (simg, smem) = texture(sheet_ext);
    let (sstg, sstg_mem) = buffer(sheet_bytes, vk::BufferUsageFlags::TRANSFER_SRC);
    let sp = dev
        .map_memory(sstg_mem, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())
        .unwrap() as *mut u8;
    let sview = view(simg, vk::Format::R8G8B8A8_UNORM);
    let sheet_version = Cell::new(u64::MAX); // nothing uploaded yet: the first `record` does
    let sampler = dev
        .create_sampler(
            &vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
                .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE),
            None,
        )
        .unwrap();
    // sets 0 (the atlas) and 1 (the image sheet): binding 0 the texture, binding 1 a sampler (WGSL
    // keeps them separate)
    let dsl = dev
        .create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::default().bindings(&[
                vk::DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                vk::DescriptorSetLayoutBinding::default()
                    .binding(1)
                    .descriptor_type(vk::DescriptorType::SAMPLER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            ]),
            None,
        )
        .unwrap();
    let dpool = dev
        .create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::default()
                .max_sets(2)
                .pool_sizes(&[
                    vk::DescriptorPoolSize::default()
                        .ty(vk::DescriptorType::SAMPLED_IMAGE)
                        .descriptor_count(2),
                    vk::DescriptorPoolSize::default()
                        .ty(vk::DescriptorType::SAMPLER)
                        .descriptor_count(2),
                ]),
            None,
        )
        .unwrap();
    let sets = dev
        .allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(dpool)
                .set_layouts(&[dsl, dsl]),
        )
        .unwrap();
    fn write<'a>(
        set: vk::DescriptorSet,
        binding: u32,
        ty: vk::DescriptorType,
        info: &'a [vk::DescriptorImageInfo],
    ) -> vk::WriteDescriptorSet<'a> {
        vk::WriteDescriptorSet::default()
            .dst_set(set)
            .dst_binding(binding)
            .descriptor_type(ty)
            .image_info(info)
    }
    let read = |v| {
        [vk::DescriptorImageInfo::default()
            .image_view(v)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)]
    };
    let (atlas_info, sheet_info) = (read(tview), read(sview));
    let samp_info = [vk::DescriptorImageInfo::default().sampler(sampler)];
    let (image, samp) = (
        vk::DescriptorType::SAMPLED_IMAGE,
        vk::DescriptorType::SAMPLER,
    );
    dev.update_descriptor_sets(
        &[
            write(sets[0], 0, image, &atlas_info),
            write(sets[0], 1, samp, &samp_info),
            write(sets[1], 0, image, &sheet_info),
            write(sets[1], 1, samp, &samp_info),
        ],
        &[],
    );

    // render pass: draw into a multisampled image, resolve it into the swapchain image
    let attach = [
        vk::AttachmentDescription::default()
            .format(fmt.format)
            .samples(SAMPLES)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL),
        vk::AttachmentDescription::default()
            .format(fmt.format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::DONT_CARE)
            .store_op(vk::AttachmentStoreOp::STORE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR),
    ];
    let color_ref = [vk::AttachmentReference::default()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];
    let resolve_ref = [vk::AttachmentReference::default()
        .attachment(1)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];
    let subpass = [vk::SubpassDescription::default()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_ref)
        .resolve_attachments(&resolve_ref)];
    let stage = vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
    let dep = [vk::SubpassDependency::default()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .src_stage_mask(stage)
        .dst_stage_mask(stage)
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)];
    let rp = dev
        .create_render_pass(
            &vk::RenderPassCreateInfo::default()
                .attachments(&attach)
                .subpasses(&subpass)
                .dependencies(&dep),
            None,
        )
        .unwrap();

    // pipelines: the shaders live in one SPIR-V module (entry points "vs", "fs_flat", "fs_text",
    // "fs_image").
    // Viewport and scissor are dynamic so a resize does not need new pipelines.
    let code = ash::util::read_spv(&mut Cursor::new(SPV)).unwrap();
    let module = dev
        .create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&code), None)
        .unwrap();
    let stages = |fs: &'static CStr| {
        [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(module)
                .name(c"vs"),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(module)
                .name(fs),
        ]
    };
    let (flat_stages, text_stages, image_stages) =
        (stages(c"fs_flat"), stages(c"fs_text"), stages(c"fs_image"));
    let pl = dev
        .create_pipeline_layout(
            &vk::PipelineLayoutCreateInfo::default().set_layouts(&[dsl, dsl]),
            None,
        )
        .unwrap();
    // straight alpha blending: the text fragments carry their coverage in alpha
    let blend = [vk::PipelineColorBlendAttachmentState::default()
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .alpha_blend_op(vk::BlendOp::ADD)
        .color_write_mask(vk::ColorComponentFlags::RGBA)];
    let binds = [vk::VertexInputBindingDescription::default()
        .stride(32)
        .input_rate(vk::VertexInputRate::VERTEX)];
    let attrs = [
        vk::VertexInputAttributeDescription::default()
            .location(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(0),
        vk::VertexInputAttributeDescription::default()
            .location(1)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(8),
        vk::VertexInputAttributeDescription::default()
            .location(2)
            .format(vk::Format::R32G32B32A32_SFLOAT)
            .offset(16),
    ];
    let vin = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&binds)
        .vertex_attribute_descriptions(&attrs);
    let ia = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
    let vp = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1);
    let dyn_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dyn_states);
    let rs = vk::PipelineRasterizationStateCreateInfo::default()
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(vk::CullModeFlags::NONE)
        .line_width(1.0);
    let ms = vk::PipelineMultisampleStateCreateInfo::default().rasterization_samples(SAMPLES);
    let cb = vk::PipelineColorBlendStateCreateInfo::default().attachments(&blend);
    let base = vk::GraphicsPipelineCreateInfo::default()
        .vertex_input_state(&vin)
        .input_assembly_state(&ia)
        .viewport_state(&vp)
        .dynamic_state(&dynamic)
        .rasterization_state(&rs)
        .multisample_state(&ms)
        .color_blend_state(&cb)
        .layout(pl)
        .render_pass(rp);
    let pipes = dev
        .create_graphics_pipelines(
            vk::PipelineCache::null(),
            &[
                base.stages(&flat_stages),
                base.stages(&text_stages),
                base.stages(&image_stages),
            ],
            None,
        )
        .unwrap();
    let (pipe_flat, pipe_text, pipe_image) = (pipes[0], pipes[1], pipes[2]);

    let clear = vk::ClearValue {
        color: vk::ClearColorValue {
            float32: [0.08, 0.08, 0.1, 1.0],
        },
    };

    // the size the surface has now
    let extent_of = || {
        let caps = surf_i
            .get_physical_device_surface_capabilities(pd, surf)
            .unwrap();
        if caps.current_extent.width != u32::MAX {
            return caps.current_extent;
        }
        let s = size.load(Relaxed);
        vk::Extent2D {
            width: ((s >> 32) as u32)
                .clamp(caps.min_image_extent.width, caps.max_image_extent.width),
            height: (s as u32).clamp(caps.min_image_extent.height, caps.max_image_extent.height),
        }
    };

    // (re)creates everything that depends on the size; the GPU must be idle. `old` is the
    // swapchain being replaced (null at first); it must still be freed by the caller.
    let build = |old: vk::SwapchainKHR, extent: vk::Extent2D| {
        let caps = surf_i
            .get_physical_device_surface_capabilities(pd, surf)
            .unwrap();
        let sc = sc_d
            .create_swapchain(
                &vk::SwapchainCreateInfoKHR::default()
                    .surface(surf)
                    .min_image_count(caps.min_image_count.max(2))
                    .image_format(fmt.format)
                    .image_color_space(fmt.color_space)
                    .image_extent(extent)
                    .image_array_layers(1)
                    .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                    .pre_transform(caps.current_transform)
                    .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                    .present_mode(vk::PresentModeKHR::FIFO)
                    .clipped(true)
                    .old_swapchain(old),
                None,
            )
            .unwrap();
        let images = sc_d.get_swapchain_images(sc).unwrap();

        // the multisampled images that the render pass draws into: colour
        let msaa_image = |format, usage| {
            let img = dev
                .create_image(
                    &vk::ImageCreateInfo::default()
                        .image_type(vk::ImageType::TYPE_2D)
                        .format(format)
                        .extent(vk::Extent3D {
                            width: extent.width,
                            height: extent.height,
                            depth: 1,
                        })
                        .mip_levels(1)
                        .array_layers(1)
                        .samples(SAMPLES)
                        .tiling(vk::ImageTiling::OPTIMAL)
                        .usage(usage)
                        .sharing_mode(vk::SharingMode::EXCLUSIVE)
                        .initial_layout(vk::ImageLayout::UNDEFINED),
                    None,
                )
                .unwrap();
            let req = dev.get_image_memory_requirements(img);
            let mem = dev
                .allocate_memory(
                    &vk::MemoryAllocateInfo::default()
                        .allocation_size(req.size)
                        .memory_type_index(mem_type(
                            req.memory_type_bits,
                            vk::MemoryPropertyFlags::DEVICE_LOCAL,
                        )),
                    None,
                )
                .unwrap();
            dev.bind_image_memory(img, mem, 0).unwrap();
            (img, mem)
        };
        let (img, imem) = msaa_image(fmt.format, vk::ImageUsageFlags::COLOR_ATTACHMENT);
        let mview = view(img, fmt.format);

        let cmds = dev
            .allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(pool)
                    .command_buffer_count(images.len() as u32),
            )
            .unwrap();
        let (mut views, mut fbs) = (vec![], vec![]);
        for &img in &images {
            let v = view(img, fmt.format);
            fbs.push(
                dev.create_framebuffer(
                    &vk::FramebufferCreateInfo::default()
                        .render_pass(rp)
                        .attachments(&[mview, v])
                        .width(extent.width)
                        .height(extent.height)
                        .layers(1),
                    None,
                )
                .unwrap(),
            );
            views.push(v);
        }
        Frame {
            sc,
            extent,
            views,
            fbs,
            cmds,
            msaa: (img, imem, mview),
        }
    };
    // destroys a `Frame`, the GPU must be idle
    let free = |f: Frame| {
        dev.free_command_buffers(pool, &f.cmds);
        f.fbs
            .into_iter()
            .for_each(|x| dev.destroy_framebuffer(x, None));
        f.views
            .into_iter()
            .for_each(|x| dev.destroy_image_view(x, None));
        dev.destroy_image_view(f.msaa.2, None);
        dev.destroy_image(f.msaa.0, None);
        dev.free_memory(f.msaa.1, None);
        sc_d.destroy_swapchain(f.sc, None);
    };

    // draws the scene for the mouse `m` (see `pack_mouse`) into the vertex buffer and records every
    // command buffer of `f` again; the GPU must be idle
    let record = |f: &Frame, m: u64| {
        let mouse = if m == NO_MOUSE {
            [f32::NAN; 2]
        } else {
            [(m >> 32) as u32 as i32 as f32, m as u32 as i32 as f32]
        };
        let s = scene(f.extent.width, f.extent.height, mouse);
        target.set_cursor(s.cursor);
        {
            // the scene may have added or dropped images: upload the sheet again (the GPU is idle)
            let im = content.images.lock().unwrap();
            if im.version != sheet_version.get() {
                sp.copy_from_nonoverlapping(im.rgba.as_ptr(), sheet_bytes);
                upload(simg, sstg, sheet_ext);
                sheet_version.set(im.version);
            }
        }
        // flat, textured, images, then the same three over them, one after the other (uv of the
        // flat ones unused)
        let parts = [
            &s.flat,
            &s.textured,
            &s.images,
            &s.over_flat,
            &s.over_textured,
            &s.over_images,
        ];
        // a scene past the buffer (a huge window) loses its tail, whole triangles only; the
        // alternative was a panic
        let mut room = MAX_VERTS;
        let n = parts.map(|p| {
            let k = p.len().min(room) / 3 * 3;
            room -= k;
            k
        });
        let mut at = 0;
        for (p, &k) in parts.iter().zip(&n) {
            verts.add(at).copy_from_nonoverlapping(p.as_ptr(), k);
            at += k;
        }
        let area = vk::Rect2D {
            offset: vk::Offset2D::default(),
            extent: f.extent,
        };
        dev.reset_command_pool(pool, vk::CommandPoolResetFlags::empty())
            .unwrap();
        for (&cmd, &fb) in f.cmds.iter().zip(&f.fbs) {
            dev.begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::default())
                .unwrap();
            dev.cmd_begin_render_pass(
                cmd,
                &vk::RenderPassBeginInfo::default()
                    .render_pass(rp)
                    .framebuffer(fb)
                    .render_area(area)
                    .clear_values(&[clear, clear]),
                vk::SubpassContents::INLINE,
            );
            dev.cmd_set_viewport(
                cmd,
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: f.extent.width as f32,
                    height: f.extent.height as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            );
            dev.cmd_set_scissor(cmd, 0, &[area]);
            dev.cmd_bind_descriptor_sets(cmd, vk::PipelineBindPoint::GRAPHICS, pl, 0, &sets, &[]);
            dev.cmd_bind_vertex_buffers(cmd, 0, &[vb], &[0]);
            // the page: flat, text, images; then the same for what goes over it
            let mut first = 0;
            for (i, &count) in n.iter().enumerate() {
                let pipe = [pipe_flat, pipe_text, pipe_image][i % 3];
                dev.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipe);
                dev.cmd_draw(cmd, count as u32, 1, first as u32, 0);
                first += count;
            }
            dev.cmd_end_render_pass(cmd);
            dev.end_command_buffer(cmd).unwrap();
        }
    };

    // frame loop
    let mut frame = build(vk::SwapchainKHR::null(), extent_of());
    let acquire = dev
        .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
        .unwrap();
    let done = dev
        .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
        .unwrap();
    let mut last = !1; // the mouse of the last recording; never a real one, so the first frame records
    while !stop.load(Relaxed) {
        let now = extent_of();
        if now.width == 0 || now.height == 0 {
            std::thread::sleep(std::time::Duration::from_millis(16)); // minimized
            continue;
        }
        let resized = now != frame.extent;
        if resized {
            dev.device_wait_idle().unwrap();
            let new = build(frame.sc, now);
            free(std::mem::replace(&mut frame, new));
        }
        // the GPU is idle here (queue_wait_idle below), so the scene can be rewritten
        let m = content.mouse.load(Relaxed);
        if content.events.dirty() || resized || m != last {
            record(&frame, m);
            last = m;
        }
        // errors (e.g. out of date mid-resize) just skip the frame; the next one rebuilds
        if let Ok((i, _)) = sc_d.acquire_next_image(frame.sc, u64::MAX, acquire, vk::Fence::null())
        {
            dev.queue_submit(
                queue,
                &[vk::SubmitInfo::default()
                    .wait_semaphores(&[acquire])
                    .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                    .command_buffers(&[frame.cmds[i as usize]])
                    .signal_semaphores(&[done])],
                vk::Fence::null(),
            )
            .unwrap();
            let r = sc_d.queue_present(
                queue,
                &vk::PresentInfoKHR::default()
                    .wait_semaphores(&[done])
                    .swapchains(&[frame.sc])
                    .image_indices(&[i]),
            );
            dev.queue_wait_idle(queue).unwrap();
            if r.is_ok() {
                presented.store(pack_size(frame.extent.width, frame.extent.height), Relaxed);
            }
        }
        if !resized {
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }

    // teardown
    dev.device_wait_idle().unwrap();
    dev.destroy_semaphore(acquire, None);
    dev.destroy_semaphore(done, None);
    free(frame);
    dev.destroy_command_pool(pool, None);
    dev.destroy_buffer(vb, None);
    dev.free_memory(vmem, None); // also unmaps
    dev.destroy_pipeline(pipe_flat, None);
    dev.destroy_pipeline(pipe_text, None);
    dev.destroy_pipeline(pipe_image, None);
    dev.destroy_pipeline_layout(pl, None);
    dev.destroy_shader_module(module, None);
    dev.destroy_render_pass(rp, None);
    dev.destroy_descriptor_pool(dpool, None);
    dev.destroy_descriptor_set_layout(dsl, None);
    dev.destroy_sampler(sampler, None);
    dev.destroy_image_view(tview, None);
    dev.destroy_image(tex, None);
    dev.free_memory(tmem, None);
    dev.destroy_image_view(sview, None);
    dev.destroy_image(simg, None);
    dev.free_memory(smem, None);
    dev.destroy_buffer(sstg, None);
    dev.free_memory(sstg_mem, None); // also unmaps
    dev.destroy_device(None);
    surf_i.destroy_surface(surf, None);
    inst.destroy_instance(None);
}
