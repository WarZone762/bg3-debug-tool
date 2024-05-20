use std::{
    collections::HashMap,
    mem::{self, ManuallyDrop},
    ptr::NonNull,
    sync::Mutex,
};

use ash::vk::{self, Handle};
use egui::{
    epaint::{self, Primitive},
    Context, FullOutput, ImageData, TextureId, TexturesDelta,
};

use crate::{
    game_definitions::FixedString,
    globals::Globals,
    hook_definitions,
    hooks::detour,
    menu::{egui_backend_win32, egui_vulkan::EguiMenu},
};

fn instance() -> &'static ash::Instance {
    unsafe { INSTANCE.as_ref().unwrap_unchecked() }
}

fn physical_dev() -> vk::PhysicalDevice {
    unsafe { PHYSICAL_DEV.unwrap_unchecked() }
}

fn dev() -> &'static ash::Device {
    unsafe { DEV.as_ref().unwrap_unchecked() }
}

fn allocator() -> &'static Allocator {
    unsafe { ALLOCATOR.as_ref().unwrap_unchecked() }
}

fn menu() -> &'static mut dyn EguiMenu {
    unsafe { MENU.as_mut().unwrap_unchecked() }
}

fn data() -> &'static VkData {
    unsafe { DATA.get_mut().unwrap().as_ref().unwrap_unchecked() }
}

static mut INSTANCE: Option<ash::Instance> = None;
static mut PHYSICAL_DEV: Option<vk::PhysicalDevice> = None;
static mut DEV: Option<ash::Device> = None;
static mut PIPELINE_CACHE: vk::PipelineCache = vk::PipelineCache::null();
static mut ALLOCATOR: Option<Allocator> = None;
static mut MENU: Option<Box<dyn EguiMenu>> = None;

static mut DATA: Mutex<Option<VkData>> = Mutex::new(None);

pub(crate) fn init(menu: impl EguiMenu + 'static) -> anyhow::Result<()> {
    unsafe {
        MENU = Some(Box::new(menu));
    }
    hook()
}

hook_definitions! {
vulkan("vulkan-1.dll") {
    fn vkCreateInstance(
        p_create_info: *mut vk::InstanceCreateInfo,
        p_allocator: *const vk::AllocationCallbacks,
        p_instance: *mut vk::Instance,
    ) -> vk::Result {
        let res = original::vkCreateInstance(p_create_info, p_allocator, p_instance);

        let instance = unsafe {
            ash::Instance::load(
                ash::Entry::load()
                    .expect("failed to load Vulkan library")
                    .static_fn(),
                *p_instance,
            )
        };
        unsafe {
            INSTANCE = Some(instance);
        }

        res
    }

    fn vkCreateDevice(
        physical_device: vk::PhysicalDevice,
        p_create_info: *const vk::DeviceCreateInfo,
        p_allocator: *const vk::AllocationCallbacks,
        p_device: *mut vk::Device,
    ) -> vk::Result {
        let res = original::vkCreateDevice(
            physical_device,
            p_create_info,
            p_allocator,
            p_device
        );

        unsafe {
            PHYSICAL_DEV = Some(physical_device);
            ALLOCATOR = Some(Allocator::new());
            DEV = Some(ash::Device::load(instance().fp_v1_0(), *p_device));

            let create_pipeline_cache = instance()
                .get_device_proc_addr(*p_device, c"vkCreatePipelineCache".as_ptr())
                .unwrap();
            let create_swapchain = instance()
                .get_device_proc_addr(*p_device, c"vkCreateSwapchainKHR".as_ptr())
                .unwrap();
            let queue_present = instance()
                .get_device_proc_addr(*p_device, c"vkQueuePresentKHR".as_ptr())
                .unwrap();

            if HOOKS.vkQueuePresentKHR.is_attached() {
                detour(|| {
                    HOOKS.vkCreatePipelineCache.detach();
                    HOOKS.vkCreateSwapchainKHR.detach();
                    HOOKS.vkQueuePresentKHR.detach();
                });
            }
            detour(|| {
                HOOKS.vkCreatePipelineCache.attach(create_pipeline_cache as _);
                HOOKS.vkCreateSwapchainKHR.attach(create_swapchain as _);
                HOOKS.vkQueuePresentKHR.attach(queue_present as _);
            });
        }

        res
    }

    #[no_init = yes]
    fn vkCreatePipelineCache(
        device: vk::Device,
        p_create_info: *const vk::PipelineCacheCreateInfo,
        p_allocator: *const vk::AllocationCallbacks,
        p_pipeline_cache: *mut vk::PipelineCache,
    ) -> vk::Result {
        let res = original::vkCreatePipelineCache(
            device,
            p_create_info,
            p_allocator,
            p_pipeline_cache
        );

        unsafe {
            PIPELINE_CACHE = *p_pipeline_cache;
        }

        res
    }

    #[no_init = yes]
    fn vkCreateSwapchainKHR(
        device: vk::Device,
        p_create_info: *const vk::SwapchainCreateInfoKHR,
        p_allocator: *const vk::AllocationCallbacks,
        p_swapchain: *mut vk::SwapchainKHR,
    ) -> vk::Result {
        let res = original::vkCreateSwapchainKHR(
            device,
            p_create_info,
            p_allocator,
            p_swapchain,
        );

        unsafe {
            let mut vk_data = DATA.lock().unwrap();
            if let Some(vk_data) = vk_data.as_mut() {
                vk_data.swapchain_data = ManuallyDrop::new(SwapchainData::new(
                    *p_swapchain,
                    &*p_create_info,
                    vk_data.cmd_pool,
                    vk_data.textures.dsc_set_layout,
                ));
            } else {
                *vk_data = Some(VkData::new(*p_swapchain, &*p_create_info));
            }
        }

        res
    }

    #[no_init = yes]
    fn vkQueuePresentKHR(
        queue: vk::Queue,
        p_present_info: *const vk::PresentInfoKHR,
    ) -> vk::Result {
        unsafe {
            if let Some(x) = &mut *DATA.lock().unwrap() {
                x.present((p_present_info as *mut vk::PresentInfoKHR).as_mut().unwrap())
            }
        }

        original::vkQueuePresentKHR(queue, p_present_info)
    }
}
}

pub(crate) struct VkData {
    queue: vk::Queue,
    cmd_pool: vk::CommandPool,
    swapchain_data: ManuallyDrop<SwapchainData>,
    textures: Textures,
    ctx: Context,
}

impl VkData {
    pub unsafe fn new(
        swapchain: vk::SwapchainKHR,
        swapchain_create_info: &vk::SwapchainCreateInfoKHR,
    ) -> Self {
        let ctx = Context::default();
        // menu.init(&mut ctx, &mut dev);
        egui_backend_win32::init();

        let families = instance().get_physical_device_queue_family_properties(physical_dev());

        let queue_family =
            families.iter().position(|x| x.queue_flags.contains(vk::QueueFlags::GRAPHICS)).unwrap()
                as _;

        let queue = dev().get_device_queue(queue_family, 0);

        let info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family);
        let cmd_pool = dev().create_command_pool(&info, None).unwrap();

        let textures = Textures::new();

        let swapchain_data = ManuallyDrop::new(SwapchainData::new(
            swapchain,
            swapchain_create_info,
            cmd_pool,
            textures.dsc_set_layout,
        ));

        Self { queue, cmd_pool, swapchain_data, textures, ctx }
    }

    unsafe fn present(&mut self, present_info: &mut vk::PresentInfoKHR) {
        if present_info.swapchain_count != 1
            || *present_info.p_swapchains != self.swapchain_data.swapchain
        {
            return;
        }

        let (full_output, screen_rect) = self.egui_run();

        let image_idx = *present_info.p_image_indices as usize;
        let image = &self.swapchain_data.images[image_idx];
        dev().wait_for_fences(&[image.fence], true, u64::MAX).unwrap();
        dev().reset_fences(&[image.fence]).unwrap();
        dev().reset_command_buffer(image.cmd_buf, vk::CommandBufferResetFlags::empty()).unwrap();

        dev()
            .begin_command_buffer(
                image.cmd_buf,
                &vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )
            .unwrap();

        let info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.swapchain_data.render_pass)
            .framebuffer(image.framebuffer)
            .render_area(*vk::Rect2D::builder().extent(self.swapchain_data.extent))
            .clear_values(&[]);
        dev().cmd_begin_render_pass(image.cmd_buf, &info, vk::SubpassContents::INLINE);

        self.swapchain_data.draw_egui(
            full_output.textures_delta,
            self.ctx.tessellate(full_output.shapes, full_output.pixels_per_point),
            image_idx,
            &mut self.textures,
            screen_rect,
        );
        let image = &self.swapchain_data.images[image_idx];

        dev().cmd_end_render_pass(image.cmd_buf);
        dev().end_command_buffer(image.cmd_buf).unwrap();

        let signal_semaphores = [image.semaphore];
        let command_buffers = [image.cmd_buf];
        let mut submit_info = vk::SubmitInfo::builder()
            .command_buffers(&command_buffers)
            .wait_dst_stage_mask(&[
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::PipelineStageFlags::ALL_COMMANDS,
            ])
            .signal_semaphores(&signal_semaphores);
        submit_info.wait_semaphore_count = present_info.wait_semaphore_count;
        submit_info.p_wait_semaphores = present_info.p_wait_semaphores;

        dev().queue_submit(self.queue, &[*submit_info], image.fence).unwrap();

        *(present_info.p_wait_semaphores as *mut _) = *submit_info.p_signal_semaphores;
        present_info.wait_semaphore_count = 1;
    }

    fn egui_run(&mut self) -> (FullOutput, egui::Rect) {
        if let Ok(input) = egui_backend_win32::new_frame() {
            let rect = input.screen_rect.unwrap();
            (self.ctx.run(input, |ctx| menu().draw(ctx)), rect)
        } else {
            (FullOutput::default(), egui::Rect::ZERO)
        }
    }
}

impl Drop for VkData {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.swapchain_data);
            dev().destroy_command_pool(self.cmd_pool, None);
        }
    }
}

#[derive(Debug, Default)]
struct Textures {
    dsc_set_layout: vk::DescriptorSetLayout,
    dsc_pool: vk::DescriptorPool,
    sampler: vk::Sampler,

    managed: HashMap<TextureId, Texture>,
    game: HashMap<u32, vk::DescriptorSet>,
}

impl Textures {
    unsafe fn new() -> Self {
        let dsc_pool = dev()
            .create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::builder()
                    .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
                    .max_sets(1024)
                    .pool_sizes(&[*vk::DescriptorPoolSize::builder()
                        .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .descriptor_count(1024)]),
                None,
            )
            .unwrap();

        let dsc_set_layout = dev()
            .create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::builder().bindings(&[
                    *vk::DescriptorSetLayoutBinding::builder()
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .descriptor_count(1)
                        .binding(0)
                        .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                ]),
                None,
            )
            .unwrap();

        let sampler = dev()
            .create_sampler(
                &vk::SamplerCreateInfo::builder()
                    .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .min_filter(vk::Filter::LINEAR)
                    .mag_filter(vk::Filter::LINEAR)
                    .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                    .max_lod(vk::LOD_CLAMP_NONE),
                None,
            )
            .unwrap();

        Self { dsc_set_layout, dsc_pool, sampler, managed: HashMap::new(), game: HashMap::new() }
    }

    unsafe fn create_texture(&mut self, id: TextureId, image_data: &ImageData) -> &mut Texture {
        let extent = vk::Extent3D {
            width: image_data.width() as _,
            height: image_data.height() as _,
            depth: 1,
        };

        let image = dev()
            .create_image(
                &vk::ImageCreateInfo::builder()
                    .extent(extent)
                    .format(vk::Format::R8G8B8A8_UNORM)
                    .image_type(vk::ImageType::TYPE_2D)
                    .array_layers(1)
                    .mip_levels(1)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST),
                None,
            )
            .unwrap();

        let requirements = dev().get_image_memory_requirements(image);
        let buf = allocator().alloc(requirements, vk::MemoryPropertyFlags::DEVICE_LOCAL);

        dev().bind_image_memory(image, buf.handle, 0).unwrap();

        let view = dev()
            .create_image_view(
                &vk::ImageViewCreateInfo::builder()
                    .components(vk::ComponentMapping::default())
                    .format(vk::Format::R8G8B8A8_UNORM)
                    .image(image)
                    .subresource_range(
                        *vk::ImageSubresourceRange::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .layer_count(1)
                            .level_count(1),
                    )
                    .view_type(vk::ImageViewType::TYPE_2D),
                None,
            )
            .unwrap();

        let dsc_set = self.create_set(view);

        let texture = Texture { image, view, dsc_set, buf };
        self.managed.insert(id, texture);

        self.managed.get_mut(&id).unwrap()
    }

    unsafe fn create_set(&mut self, view: vk::ImageView) -> vk::DescriptorSet {
        let dsc_set = dev()
            .allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(self.dsc_pool)
                    .set_layouts(&[self.dsc_set_layout]),
            )
            .unwrap()[0];
        dev().update_descriptor_sets(
            &[*vk::WriteDescriptorSet::builder()
                .dst_set(dsc_set)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&[*vk::DescriptorImageInfo::builder()
                    .image_view(view)
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .sampler(self.sampler)])],
            &[],
        );

        dsc_set
    }

    fn free_texture(&mut self, id: TextureId) {
        self.managed.remove_entry(&id);
    }
}

impl Drop for Textures {
    fn drop(&mut self) {
        unsafe {
            dev().destroy_sampler(self.sampler, None);
            dev().destroy_descriptor_pool(self.dsc_pool, None);
            dev().destroy_descriptor_set_layout(self.dsc_set_layout, None);
        }
    }
}

#[derive(Debug)]
struct Texture {
    image: vk::Image,
    view: vk::ImageView,
    dsc_set: vk::DescriptorSet,
    buf: Allocation<u8>,
}

impl Texture {
    fn apply_delta(&self, delta: &epaint::ImageDelta, cmd_buf: vk::CommandBuffer) -> Buffer<u8> {
        let data = match &delta.image {
            ImageData::Color(image) => {
                image.pixels.iter().flat_map(|c| c.to_array()).collect::<Vec<_>>()
            }
            ImageData::Font(image) => {
                image.srgba_pixels(None).flat_map(|c| c.to_array()).collect::<Vec<_>>()
            }
        };

        let mut buf = Buffer::<u8>::new(
            data.len(),
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::SharingMode::default(),
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        buf.as_slice_mut().copy_from_slice(&data);
        unsafe {
            dev().cmd_copy_buffer_to_image(
                cmd_buf,
                buf.buf,
                self.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[*vk::BufferImageCopy::builder()
                    .buffer_row_length(delta.image.width() as _)
                    .buffer_image_height(delta.image.height() as _)
                    .image_subresource(
                        *vk::ImageSubresourceLayers::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .layer_count(1),
                    )
                    .image_offset(
                        delta
                            .pos
                            .map(|pos| vk::Offset3D { x: pos[0] as _, y: pos[1] as _, z: 0 })
                            .unwrap_or_default(),
                    )
                    .image_extent(vk::Extent3D {
                        width: delta.image.width() as _,
                        height: delta.image.height() as _,
                        depth: 1,
                    })],
            );
        }
        buf
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            dev().destroy_image_view(self.view, None);
            dev().destroy_image(self.image, None);
            dev().free_descriptor_sets(data().textures.dsc_pool, &[self.dsc_set]).unwrap();
        }
        allocator().free(&mut self.buf)
    }
}

#[derive(Debug)]
struct SwapchainData {
    swapchain: vk::SwapchainKHR,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    images: Vec<SwapchainImage>,
    extent: vk::Extent2D,
}

impl SwapchainData {
    pub unsafe fn new(
        swapchain: vk::SwapchainKHR,
        create_info: &vk::SwapchainCreateInfoKHR,
        cmd_pool: vk::CommandPool,
        dsc_set_layout: vk::DescriptorSetLayout,
    ) -> Self {
        let extent = create_info.image_extent;

        let attachment = vk::AttachmentDescription::builder()
            .format(create_info.image_format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::LOAD)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

        let color_attachment =
            vk::AttachmentReference::builder().layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
        let color_attachments = [*color_attachment];

        let subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachments);

        let dependency = vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT);

        let attachments = [*attachment];
        let subpasses = [*subpass];
        let dependencies = [*dependency];

        let render_pass = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);

        let render_pass = dev().create_render_pass(&render_pass, None).unwrap();

        let pipeline_layout = dev()
            .create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(&[dsc_set_layout])
                    .push_constant_ranges(&[*vk::PushConstantRange::builder()
                        .stage_flags(vk::ShaderStageFlags::VERTEX)
                        .offset(0)
                        .size(mem::size_of::<f32>() as u32 * 2)]),
                None,
            )
            .unwrap();

        let pipeline = Self::create_pipeline(pipeline_layout, render_pass);

        let swapchain_khr = ash::extensions::khr::Swapchain::new(instance(), dev());
        let images = swapchain_khr
            .get_swapchain_images(swapchain)
            .unwrap()
            .into_iter()
            .map(|image| {
                SwapchainImage::new(image, cmd_pool, render_pass, create_info.image_format, extent)
            })
            .collect();

        Self { swapchain, render_pass, pipeline_layout, pipeline, images, extent }
    }

    fn create_pipeline(
        pipeline_layout: vk::PipelineLayout,
        render_pass: vk::RenderPass,
    ) -> vk::Pipeline {
        let attributes = [
            *vk::VertexInputAttributeDescription::builder()
                .offset(0)
                .location(0)
                .format(vk::Format::R32G32_SFLOAT),
            *vk::VertexInputAttributeDescription::builder()
                .offset(8)
                .location(1)
                .format(vk::Format::R32G32_SFLOAT),
            *vk::VertexInputAttributeDescription::builder()
                .offset(16)
                .location(2)
                .format(vk::Format::R8G8B8A8_UNORM),
        ];

        let bytes_code = include_bytes!("shaders/vert.spv");
        let info = vk::ShaderModuleCreateInfo {
            code_size: bytes_code.len(),
            p_code: bytes_code.as_ptr() as _,
            ..Default::default()
        };
        let vertex_shader_mod = unsafe { dev().create_shader_module(&info, None).unwrap() };

        let bytes_code = include_bytes!("shaders/frag.spv");
        let info = vk::ShaderModuleCreateInfo {
            code_size: bytes_code.len(),
            p_code: bytes_code.as_ptr() as _,
            ..Default::default()
        };
        let fragment_shader_mod = unsafe { dev().create_shader_module(&info, None).unwrap() };

        let main_function_name = c"main";
        let pipeline_shader_stages = [
            *vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vertex_shader_mod)
                .name(main_function_name),
            *vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(fragment_shader_mod)
                .name(main_function_name),
        ];

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
        let viewport_info =
            vk::PipelineViewportStateCreateInfo::builder().viewport_count(1).scissor_count(1);
        let rasterization_info =
            vk::PipelineRasterizationStateCreateInfo::builder().line_width(1.0);
        let depth_stencil_info = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_compare_op(vk::CompareOp::ALWAYS)
            .front(*vk::StencilOpState::builder().compare_op(vk::CompareOp::ALWAYS))
            .back(*vk::StencilOpState::builder().compare_op(vk::CompareOp::ALWAYS));
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states);
        let multisample_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let pipeline = unsafe {
            dev()
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[*vk::GraphicsPipelineCreateInfo::builder()
                        .stages(&pipeline_shader_stages)
                        .vertex_input_state(
                            &vk::PipelineVertexInputStateCreateInfo::builder()
                                .vertex_attribute_descriptions(&attributes)
                                .vertex_binding_descriptions(&[
                                    *vk::VertexInputBindingDescription::builder()
                                        .binding(0)
                                        .input_rate(vk::VertexInputRate::VERTEX)
                                        .stride(
                                            4 * mem::size_of::<f32>() as u32
                                                + 4 * mem::size_of::<u8>() as u32,
                                        ),
                                ]),
                        )
                        .input_assembly_state(&input_assembly_info)
                        .viewport_state(&viewport_info)
                        .rasterization_state(&rasterization_info)
                        .multisample_state(&multisample_info)
                        .depth_stencil_state(&depth_stencil_info)
                        .color_blend_state(
                            &vk::PipelineColorBlendStateCreateInfo::builder().attachments(&[
                                *vk::PipelineColorBlendAttachmentState::builder()
                                    .color_write_mask(vk::ColorComponentFlags::RGBA)
                                    .blend_enable(true)
                                    .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                                    .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                                    .src_alpha_blend_factor(vk::BlendFactor::ONE)
                                    .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA),
                            ]),
                        )
                        .dynamic_state(&dynamic_state_info)
                        .layout(pipeline_layout)
                        .render_pass(render_pass)],
                    None,
                )
                .unwrap()[0]
        };
        unsafe {
            dev().destroy_shader_module(vertex_shader_mod, None);
            dev().destroy_shader_module(fragment_shader_mod, None);
        }
        pipeline
    }

    unsafe fn update_textures(
        &self,
        textures_delta: TexturesDelta,
        textures: &mut Textures,
        cmd_buf: vk::CommandBuffer,
        frame_end_fence: vk::Fence,
    ) {
        for id in textures_delta.free {
            textures.free_texture(id);
        }

        let mut staging_bufs = Vec::with_capacity(textures_delta.set.len());

        for (id, delta) in textures_delta.set {
            if delta.is_whole() {
                let texture = textures.create_texture(id, &delta.image);
                staging_bufs.push(texture.apply_delta(&delta, cmd_buf));
            } else {
                staging_bufs.push(textures.managed[&id].apply_delta(&delta, cmd_buf));
            }
        }

        if !staging_bufs.is_empty() {
            struct Wrapper<T>(T);
            unsafe impl<T> Send for Wrapper<T> {}
            unsafe impl<T> Sync for Wrapper<T> {}
            let staging_bufs = Wrapper(staging_bufs);
            std::thread::spawn(move || {
                let mut _staging_bufs = staging_bufs;
                dev().wait_for_fences(&[frame_end_fence], true, u64::MAX).unwrap();
            });
        }
    }

    unsafe fn draw_egui(
        &mut self,
        textures_delta: TexturesDelta,
        primitives: Vec<egui::ClippedPrimitive>,
        image_idx: usize,
        textures: &mut Textures,
        screen_rect: egui::Rect,
    ) {
        let image = &self.images[image_idx];

        dev().cmd_bind_pipeline(image.cmd_buf, vk::PipelineBindPoint::GRAPHICS, self.pipeline);
        dev().cmd_bind_index_buffer(image.cmd_buf, image.idx_buf.buf, 0, vk::IndexType::UINT32);
        dev().cmd_bind_vertex_buffers(image.cmd_buf, 0, &[image.vtx_buf.buf], &[0]);

        dev().cmd_push_constants(
            image.cmd_buf,
            self.pipeline_layout,
            vk::ShaderStageFlags::VERTEX,
            0,
            &screen_rect.width().to_ne_bytes(),
        );
        dev().cmd_push_constants(
            image.cmd_buf,
            self.pipeline_layout,
            vk::ShaderStageFlags::VERTEX,
            4,
            &screen_rect.height().to_ne_bytes(),
        );

        self.update_textures(textures_delta, textures, image.cmd_buf, image.fence);

        let image = &mut self.images[image_idx];
        let idx_buf = image.idx_buf.as_slice_mut();
        let vtx_buf = image.vtx_buf.as_slice_mut();

        let mut idx_base = 0;
        let mut vtx_base = 0;
        for primitive in primitives {
            match primitive.primitive {
                Primitive::Mesh(mesh) => {
                    match mesh.texture_id {
                        TextureId::Managed(_) => {
                            dev().cmd_bind_descriptor_sets(
                                image.cmd_buf,
                                vk::PipelineBindPoint::GRAPHICS,
                                self.pipeline_layout,
                                0,
                                &[textures.managed.get(&mesh.texture_id).unwrap().dsc_set],
                                &[],
                            );
                        }
                        TextureId::User(id) => {
                            if let Some(texture_manager) = Globals::static_symbols()
                                .ls__gGlobalResourceManager
                                .and_then(|x| x.as_opt())
                                .and_then(|x| x.as_opt())
                                .and_then(|x| x.texture_manager.as_opt())
                                && let Some(atlases) = Globals::static_symbols()
                                    .ls__gTextureAtlasMap
                                    .and_then(|x| x.as_opt())
                                    .and_then(|x| x.as_opt())
                            {
                                let fstring = FixedString { index: id as _ };
                                let Some(atlas_fstring) = atlases
                                    .icon_map
                                    .iter()
                                    .find(|x| x.key == fstring)
                                    .map(|x| x.value.name)
                                else {
                                    return;
                                };
                                let atlas = atlas_fstring.index;

                                if let Some(dsc_set) =
                                    textures.game.get(&atlas).copied().or_else(|| {
                                        let view = texture_manager
                                            .find(atlas_fstring)?
                                            .vulkan
                                            .image_views
                                            .first()?
                                            .view;

                                        let dsc_set =
                                            textures.create_set(vk::ImageView::from_raw(view as _));
                                        textures.game.insert(atlas, dsc_set);
                                        Some(dsc_set)
                                    })
                                {
                                    dev().cmd_bind_descriptor_sets(
                                        image.cmd_buf,
                                        vk::PipelineBindPoint::GRAPHICS,
                                        self.pipeline_layout,
                                        0,
                                        &[dsc_set],
                                        &[],
                                    );
                                } else {
                                    return;
                                }
                            }
                        }
                    }

                    idx_buf[idx_base..idx_base + mesh.indices.len()].copy_from_slice(&mesh.indices);
                    vtx_buf[vtx_base..vtx_base + mesh.vertices.len()]
                        .copy_from_slice(&mesh.vertices);

                    let clip_rect = primitive.clip_rect;
                    let min = clip_rect.clamp(screen_rect.min);
                    let max = clip_rect.clamp(screen_rect.max);
                    dev().cmd_set_scissor(image.cmd_buf, 0, &[*vk::Rect2D::builder()
                        .offset(vk::Offset2D { x: min.x.round() as i32, y: min.y.round() as i32 })
                        .extent(vk::Extent2D {
                            width: (max.x.round() - min.x) as u32,
                            height: (max.y.round() - min.y) as u32,
                        })]);
                    dev().cmd_set_viewport(image.cmd_buf, 0, &[*vk::Viewport::builder()
                        .width(screen_rect.width())
                        .height(screen_rect.height())
                        .max_depth(1.0)]);
                    dev().cmd_draw_indexed(
                        image.cmd_buf,
                        mesh.indices.len() as _,
                        1,
                        idx_base as _,
                        vtx_base as _,
                        0,
                    );
                    idx_base += mesh.indices.len();
                    vtx_base += mesh.vertices.len();
                }
                Primitive::Callback(_) => unimplemented!(),
            }
        }
    }
}

impl Drop for SwapchainData {
    fn drop(&mut self) {
        unsafe {
            dev().destroy_pipeline_layout(self.pipeline_layout, None);
            dev().destroy_pipeline(self.pipeline, None);
            dev().destroy_render_pass(self.render_pass, None);
        }
    }
}

#[derive(Debug)]
struct SwapchainImage {
    framebuffer: vk::Framebuffer,
    image_view: vk::ImageView,
    idx_buf: Buffer<u32>,
    vtx_buf: Buffer<epaint::Vertex>,
    cmd_buf: vk::CommandBuffer,
    fence: vk::Fence,
    semaphore: vk::Semaphore,
}

impl SwapchainImage {
    unsafe fn new(
        image: vk::Image,
        command_pool: vk::CommandPool,
        render_pass: vk::RenderPass,
        format: vk::Format,
        extent: vk::Extent2D,
    ) -> Self {
        let info = vk::CommandBufferAllocateInfo::builder()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(command_pool)
            .command_buffer_count(1);
        let cmd_buf = dev().allocate_command_buffers(&info).unwrap()[0];

        let info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
        let fence = dev().create_fence(&info, None).unwrap();

        let info = vk::SemaphoreCreateInfo::default();
        let semaphore = dev().create_semaphore(&info, None).unwrap();

        let info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            })
            .subresource_range(
                *vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .level_count(1)
                    .layer_count(1),
            );
        let image_view = dev().create_image_view(&info, None).unwrap();

        let attachments = [image_view];
        let info = vk::FramebufferCreateInfo::builder()
            .render_pass(render_pass)
            .attachments(&attachments)
            .width(extent.width)
            .height(extent.height)
            .layers(1);

        let framebuffer = dev().create_framebuffer(&info, None).unwrap();

        let idx_buf = Buffer::new(
            1024 * 1024 * 4,
            vk::BufferUsageFlags::INDEX_BUFFER,
            vk::SharingMode::EXCLUSIVE,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        let vtx_buf = Buffer::new(
            1024 * 1024 * 4,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::SharingMode::EXCLUSIVE,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        Self { framebuffer, image_view, idx_buf, vtx_buf, cmd_buf, fence, semaphore }
    }
}

impl Drop for SwapchainImage {
    fn drop(&mut self) {
        unsafe {
            dev().destroy_framebuffer(self.framebuffer, None);
            dev().destroy_image_view(self.image_view, None);
            dev().free_command_buffers(data().cmd_pool, &[self.cmd_buf]);
            dev().destroy_fence(self.fence, None);
            dev().destroy_semaphore(self.semaphore, None);
        }
    }
}

#[derive(Debug)]
pub(crate) struct Buffer<T> {
    buf: vk::Buffer,
    mem: Allocation<T>,
}

impl<T> Buffer<T> {
    fn new(
        size: usize,
        usage: vk::BufferUsageFlags,
        sharing_mode: vk::SharingMode,
        mem_props: vk::MemoryPropertyFlags,
    ) -> Self {
        let buffer_info =
            vk::BufferCreateInfo::builder().size(size as _).usage(usage).sharing_mode(sharing_mode);
        let buf = unsafe { dev().create_buffer(&buffer_info, None).unwrap() };

        let requirements = unsafe { dev().get_buffer_memory_requirements(buf) };

        let mem = allocator().alloc(requirements, mem_props);

        unsafe { dev().bind_buffer_memory(buf, mem.handle, 0).unwrap() };

        Self { buf, mem }
    }

    fn as_slice_mut(&mut self) -> &mut [T] {
        self.mem.as_slice_mut()
    }
}

impl<T> Drop for Buffer<T> {
    fn drop(&mut self) {
        unsafe {
            dev().destroy_buffer(self.buf, None);
        }
        allocator().free(&mut self.mem);
    }
}

#[derive(Debug)]
pub(crate) struct Allocator {
    physical_dev_mem_props: vk::PhysicalDeviceMemoryProperties,
}

impl Allocator {
    pub fn new() -> Self {
        let physical_dev_mem_props =
            unsafe { instance().get_physical_device_memory_properties(physical_dev()) };
        Self { physical_dev_mem_props }
    }

    pub fn alloc<T>(
        &self,
        requirements: vk::MemoryRequirements,
        memory_property_flags: vk::MemoryPropertyFlags,
    ) -> Allocation<T> {
        let info = vk::MemoryAllocateInfo::builder()
            .allocation_size(requirements.size)
            .memory_type_index(self.mem_type(requirements, memory_property_flags).unwrap());
        let handle = unsafe { dev().allocate_memory(&info, None).unwrap() };
        let size = requirements.size;

        let is_mapped = !memory_property_flags.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL);
        let ptr = if is_mapped {
            unsafe {
                Some(
                    NonNull::new(
                        dev().map_memory(handle, 0, size, vk::MemoryMapFlags::empty()).unwrap()
                            as _,
                    )
                    .unwrap(),
                )
            }
        } else {
            None
        };
        Allocation { handle, size: size / mem::size_of::<T>() as u64, ptr }
    }

    pub fn free<T>(&self, allocation: &mut Allocation<T>) {
        unsafe {
            if allocation.ptr.is_some() {
                dev().unmap_memory(allocation.handle)
            };
            dev().free_memory(allocation.handle, None);
        }
    }

    fn mem_type(
        &self,
        requirements: vk::MemoryRequirements,
        property_flags: vk::MemoryPropertyFlags,
    ) -> Option<u32> {
        for i in 0..(self.physical_dev_mem_props.memory_type_count as usize) {
            if (1 << i) & requirements.memory_type_bits != 0
                && self.physical_dev_mem_props.memory_types[i]
                    .property_flags
                    .contains(property_flags)
            {
                return Some(i as _);
            }
        }
        None
    }
}

#[derive(Debug)]
pub(crate) struct Allocation<T> {
    handle: vk::DeviceMemory,
    size: u64,
    /// [`None`] if allocation is not mapped
    ptr: Option<NonNull<T>>,
}

impl<T> Allocation<T> {
    fn as_slice_mut(&mut self) -> &mut [T] {
        let ptr = self.ptr.expect("allocation is not mapped");
        unsafe { std::slice::from_raw_parts_mut(ptr.as_ptr(), self.size as usize) }
    }
}
