use std::{collections::HashMap, mem, ptr, sync::Mutex};

use ash::vk;
use egui::{
    epaint::{self, Primitive},
    Context, FullOutput, ImageData, Pos2, TextureId, TexturesDelta,
};

use crate::{
    hook_definitions,
    hooks::detour,
    info,
    menu::{egui_backend_win32, egui_vulkan::EguiMenu},
};

static mut DATA: Mutex<Option<VulkanData<Box<dyn EguiMenu>>>> = Mutex::new(None);
static mut DATA_BUILDER: Mutex<VulkanDataBuilder<Box<dyn EguiMenu>>> =
    Mutex::new(VulkanDataBuilder::new());

pub(crate) fn init(menu: impl EguiMenu + 'static) -> anyhow::Result<()> {
    unsafe {
        DATA_BUILDER.lock().unwrap().menu = Some(Box::new(menu));
    }
    hook()
}

static ATLASES: Mutex<Vec<vk::Image>> = Mutex::new(Vec::new());
static ATLAS_VIEWS: Mutex<Vec<vk::ImageView>> = Mutex::new(Vec::new());
static DSC_SETS: Mutex<Vec<vk::DescriptorSet>> = Mutex::new(Vec::new());
static mut SLIDER: usize = 0;
static mut SIZE: f32 = 512.0;

hook_definitions! {
vulkan("vulkan-1.dll") {
    #[no_init = yes]
    fn vkUpdateDescriptorSets(
        device: vk::Device,
        descriptor_write_count: u32,
        p_descriptor_writes: *const vk::WriteDescriptorSet,
        descriptor_copy_count: u32,
        p_descriptor_copies: *const vk::CopyDescriptorSet,
    ) -> vk::Result {
        // info!("upd dsc sets, writes: {descriptor_write_count}, copies: {descriptor_copy_count}");
        // unsafe {
        //     let views = ATLAS_VIEWS.lock().unwrap();
        //     let mut dsc_sets = DSC_SETS.lock().unwrap();
        //     for i in 0..descriptor_write_count {
        //         let writes = *p_descriptor_writes.add(i as _);
        //         // info!("set: {:?}", writes.dst_set);
        //         // info!("binding: {}", writes.dst_binding);
        //         // info!("array element: {}", writes.dst_array_element);
        //         // info!("count: {}", writes.descriptor_count);
        //         // info!("type: {:?}", writes.descriptor_type);
        //         if !writes.p_image_info.is_null()
        //             && writes.descriptor_type == vk::DescriptorType::COMBINED_IMAGE_SAMPLER
        //         {
        //             for j in 0..writes.descriptor_count {
        //                 let info = *writes.p_image_info.add(j as _);
        //                 if info.image_layout == vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        //                     && views.iter().any(|x| *x == info.image_view)
        //                 {
        //                     info!("texture atlas descriptor set: {:?}", writes.dst_set);
        //                     dsc_sets.push(writes.dst_set);
        //                 }
        //                 // info!("image sampler: {:?}", info.sampler);
        //                 // info!("image view: {:?}", info.image_view);
        //                 // info!("image layout: {:?}", info.image_layout);
        //             }
        //         }
        //         // if !writes.p_buffer_info.is_null() {
        //         //     for j in 0..writes.descriptor_count {
        //         //         let info = *writes.p_buffer_info.add(j as _);
        //         //         info!("buffer: {:?}", info.buffer);
        //         //         info!("buffer range: {}", info.range);
        //         //         info!("buffer offset: {}", info.offset);
        //         //     }
        //         // }
        //         // if !writes.p_texel_buffer_view.is_null() {
        //         //     for j in 0..writes.descriptor_count {
        //         //         info!("buffer view: {:?}", (*writes.p_texel_buffer_view.add(j as _)));
        //         //     }
        //         // }
        //     }
        // }

        original::vkUpdateDescriptorSets(
            device,
            descriptor_write_count,
            p_descriptor_writes,
            descriptor_copy_count,
            p_descriptor_copies,
        )
    }

    #[no_init = yes]
    fn vkCreateImage(
        device: vk::Device,
        p_create_info: *const vk::ImageCreateInfo,
        p_allocator: *const vk::AllocationCallbacks,
        p_image: *mut vk::Image
    ) -> vk::Result {
        let res = original::vkCreateImage(device, p_create_info, p_allocator, p_image);
        unsafe {
            let info = *p_create_info;
            if info.image_type == vk::ImageType::TYPE_2D
                && info.format == vk::Format::BC3_UNORM_BLOCK
                && info.samples == vk::SampleCountFlags::TYPE_1
                && info.tiling == vk::ImageTiling::OPTIMAL
                && info.usage == (
                    vk::ImageUsageFlags::TRANSFER_SRC
                    | vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::SAMPLED
                )
                && info.sharing_mode == vk::SharingMode::CONCURRENT
                && info.initial_layout == vk::ImageLayout::UNDEFINED
                && info.extent.width == 2048
                && info.extent.height == 2048
                && info.extent.depth == 1
            {
                ATLASES.lock().unwrap().push(*p_image);
                // info!("possible texture atlas: {:?}", *p_image);
            }
        }

        res
    }

    #[no_init = yes]
    fn vkCreateImageView(
        device: vk::Device,
        p_create_info: *const vk::ImageViewCreateInfo,
        p_allocator: *const vk::AllocationCallbacks,
        p_image_view: *mut vk::ImageView
    ) -> vk::Result {
        let res = original::vkCreateImageView(device, p_create_info, p_allocator, p_image_view);
        unsafe {
            let info = *p_create_info;
            let atlases = ATLASES.lock().unwrap();
            if atlases.iter().any(|x| *x == info.image) {
                ATLAS_VIEWS.lock().unwrap().push(*p_image_view);
            }
        }

        res
    }

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
            if let Some(vk_data) = DATA.lock().unwrap().as_mut() {
                vk_data.instance = instance;
            } else {
                DATA_BUILDER.lock().unwrap().instance = Some(instance);
            }
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
            if let Some(vk_data) = DATA.lock().unwrap().as_mut() {
                vk_data.physical_dev = physical_device;
                vk_data.dev = ash::Device::load(vk_data.instance.fp_v1_0(), *p_device);

                let families = vk_data
                    .instance
                    .get_physical_device_queue_family_properties(
                        vk_data.physical_dev
                    );

                vk_data.queue_family = families
                    .iter()
                    .position(|x| x.queue_flags.contains(vk::QueueFlags::GRAPHICS))
                    .unwrap() as _;

                let create_pipeline_cache = vk_data
                    .instance
                    .get_device_proc_addr(*p_device, c"vkCreatePipelineCache".as_ptr())
                    .unwrap();
                let create_swapchain = vk_data
                    .instance
                    .get_device_proc_addr(*p_device, c"vkCreateSwapchainKHR".as_ptr())
                    .unwrap();
                let queue_present = vk_data
                    .instance
                    .get_device_proc_addr(*p_device, c"vkQueuePresentKHR".as_ptr())
                    .unwrap();

                detour(|| {
                    HOOKS.vkCreatePipelineCache.detach();
                    HOOKS.vkCreateSwapchainKHR.detach();
                    HOOKS.vkQueuePresentKHR.detach();
                });
                detour(|| {
                    HOOKS.vkCreatePipelineCache.attach(create_pipeline_cache as _);
                    HOOKS.vkCreateSwapchainKHR.attach(create_swapchain as _);
                    HOOKS.vkQueuePresentKHR.attach(queue_present as _);
                });
            } else {
                let mut builder = DATA_BUILDER.lock().unwrap();
                builder.physical_dev = Some(physical_device);
                builder.dev = Some(ash::Device::load(builder.instance().fp_v1_0(), *p_device));

                let families = builder
                    .instance()
                    .get_physical_device_queue_family_properties(
                        builder.physical_dev.unwrap()
                    );

                builder.queue_family = Some(
                    families
                        .iter()
                        .position(|x| x.queue_flags.contains(vk::QueueFlags::GRAPHICS))
                        .unwrap() as _,
                    );

                let create_pipeline_cache = builder
                    .instance()
                    .get_device_proc_addr(*p_device, c"vkCreatePipelineCache".as_ptr())
                    .unwrap();
                let create_swapchain = builder
                    .instance()
                    .get_device_proc_addr(*p_device, c"vkCreateSwapchainKHR".as_ptr())
                    .unwrap();
                let queue_present = builder
                    .instance()
                    .get_device_proc_addr(*p_device, c"vkQueuePresentKHR".as_ptr())
                    .unwrap();

                let upd_dsc_set = builder
                    .instance()
                    .get_device_proc_addr(*p_device, c"vkUpdateDescriptorSets".as_ptr())
                    .unwrap();
                let create_image = builder
                    .instance()
                    .get_device_proc_addr(*p_device, c"vkCreateImage".as_ptr())
                    .unwrap();
                let create_image_view = builder
                    .instance()
                    .get_device_proc_addr(*p_device, c"vkCreateImageView".as_ptr())
                    .unwrap();

                detour(|| {
                    HOOKS.vkCreatePipelineCache.attach(create_pipeline_cache as _);
                    HOOKS.vkCreateSwapchainKHR.attach(create_swapchain as _);
                    HOOKS.vkQueuePresentKHR.attach(queue_present as _);

                    HOOKS.vkUpdateDescriptorSets.attach(upd_dsc_set as _);
                    HOOKS.vkCreateImage.attach(create_image as _);
                    HOOKS.vkCreateImageView.attach(create_image_view as _);
                });
            }
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
            if let Some(vk_data) = DATA.lock().unwrap().as_mut() {
                vk_data.pipeline_cache = *p_pipeline_cache
            } else {
                DATA_BUILDER.lock().unwrap().pipeline_cache = Some(*p_pipeline_cache);
            }
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
                let old = mem::replace(
                    &mut vk_data.swapchain_data,
                    SwapchainData::new(
                        *p_swapchain,
                        &*p_create_info,
                        &vk_data.instance,
                        &vk_data.dev,
                        vk_data.queue_family,
                    )
                );
                old.destroy(&vk_data.dev);
            } else {
                let mut builder = DATA_BUILDER.lock().unwrap();
                builder.swapchain_data = Some(SwapchainData::new(
                    *p_swapchain,
                    &*p_create_info,
                    builder.instance(),
                    builder.dev.as_ref().unwrap(),
                    builder.queue_family.unwrap(),
                ));

                *vk_data = Some(builder.build());
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

pub(crate) struct VulkanData<M: EguiMenu> {
    instance: ash::Instance,
    physical_dev: vk::PhysicalDevice,
    dev: ash::Device,
    queue_family: u32,
    queue: vk::Queue,
    pipeline_cache: vk::PipelineCache,
    descriptor_pool: vk::DescriptorPool,
    swapchain_data: SwapchainData,

    menu: M,

    ctx: Context,
    allocator: Allocator,
    index_buf: Buffer,
    vertex_buf: Buffer,

    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,

    textures: HashMap<TextureId, (vk::DescriptorSet, Texture)>,

    descriptor_set_layout: vk::DescriptorSetLayout,
    sampler: vk::Sampler,
}

impl<M: EguiMenu> VulkanData<M> {
    fn present(&mut self, present_info: &mut vk::PresentInfoKHR) {
        unsafe {
            if present_info.swapchain_count != 1
                || *present_info.p_swapchains != self.swapchain_data.swapchain
            {
                return;
            }

            let full_output = self.egui_run();

            let image = self.swapchain_data.images[*present_info.p_image_indices as usize];
            self.dev.wait_for_fences(&[image.fence], true, u64::MAX).unwrap();
            self.dev.reset_fences(&[image.fence]).unwrap();
            self.dev
                .reset_command_buffer(image.command_buffer, vk::CommandBufferResetFlags::empty())
                .unwrap();

            self.dev
                .begin_command_buffer(
                    image.command_buffer,
                    &vk::CommandBufferBeginInfo::builder()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )
                .unwrap();

            let info = vk::RenderPassBeginInfo::builder()
                .render_pass(self.swapchain_data.render_pass)
                .framebuffer(image.framebuffer)
                .render_area(*vk::Rect2D::builder().extent(self.swapchain_data.extent))
                .clear_values(&[]);
            self.dev.cmd_begin_render_pass(
                image.command_buffer,
                &info,
                vk::SubpassContents::INLINE,
            );

            self.draw_egui(full_output, &image);

            self.dev.cmd_end_render_pass(image.command_buffer);
            self.dev.end_command_buffer(image.command_buffer).unwrap();

            let signal_semaphores = [image.semaphore];
            let command_buffers = [image.command_buffer];
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

            self.dev.queue_submit(self.queue, &[*submit_info], image.fence).unwrap();

            *(present_info.p_wait_semaphores as *mut _) = *submit_info.p_signal_semaphores;
            present_info.wait_semaphore_count = 1;
        }
    }

    fn egui_run(&mut self) -> FullOutput {
        if let Ok(input) = egui_backend_win32::new_frame() {
            self.ctx.run(
                input,
                // egui::RawInput {
                //     focused: true,
                //     screen_rect: Some(egui::Rect::from_min_size(
                //         Default::default(),
                //         [1920.0, 1080.0].into(),
                //     )),
                //     viewport_id: egui::ViewportId::ROOT,
                //     viewports: HashMap::from_iter([(egui::ViewportId::ROOT, egui::ViewportInfo {
                //         focused: Some(true),
                //         inner_rect: Some(egui::Rect::from_min_size(
                //             Default::default(),
                //             [1920.0, 1080.0].into(),
                //         )),
                //         outer_rect: Some(egui::Rect::from_min_size(
                //             Default::default(),
                //             [1920.0, 1080.0].into(),
                //         )),
                //         monitor_size: Some([1920.0, 1080.0].into()),
                //         ..Default::default()
                //     })]),
                //     ..Default::default()
                // },
                // |ctx| self.menu.draw(ctx),
                |ctx| {
                    egui::Window::new("egui Test").scroll2(true).max_size([512.0, 512.0]).show(
                        ctx,
                        |ui| unsafe {
                            ui.input(|i| SIZE = (SIZE * i.zoom_delta()).clamp(128.0, 4096.0));
                            ui.add(egui::Slider::new(
                                &mut SLIDER,
                                0..=(DSC_SETS.lock().unwrap().len() - 1),
                            ));
                            // let (_, rect) = ui.allocate_space(ui.available_size());
                            egui::ScrollArea::both().show(ui, |ui| {
                                ui.image((
                                    egui::TextureId::User(SLIDER as _),
                                    egui::Vec2::new(SIZE, SIZE),
                                ));
                            })
                        },
                    );
                },
            )
        } else {
            FullOutput::default()
        }
    }

    unsafe fn draw_egui(&mut self, full_output: FullOutput, image: &SwapchainImageData) {
        self.dev.cmd_bind_pipeline(
            image.command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline,
        );
        self.dev.cmd_bind_vertex_buffers(image.command_buffer, 0, &[self.vertex_buf.buf], &[0]);
        self.dev.cmd_bind_index_buffer(
            image.command_buffer,
            self.index_buf.buf,
            0,
            vk::IndexType::UINT32,
        );

        self.dev.cmd_push_constants(
            image.command_buffer,
            self.pipeline_layout,
            vk::ShaderStageFlags::VERTEX,
            0,
            &1920f32.to_ne_bytes(),
        );
        self.dev.cmd_push_constants(
            image.command_buffer,
            self.pipeline_layout,
            vk::ShaderStageFlags::VERTEX,
            4,
            &1080f32.to_ne_bytes(),
        );

        let clipped_primitives =
            self.ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

        let index_buf_ptr = self.index_buf.mem.ptr as *mut u32;
        let vertex_buf_ptr = self.vertex_buf.mem.ptr as *mut epaint::Vertex;

        self.update_textures(full_output.textures_delta, image.command_buffer);

        let mut index_base = 0;
        let mut vertex_base = 0;
        for primitive in clipped_primitives {
            match primitive.primitive {
                Primitive::Mesh(mesh) => {
                    match mesh.texture_id {
                        TextureId::Managed(_) => {
                            self.dev.cmd_bind_descriptor_sets(
                                image.command_buffer,
                                vk::PipelineBindPoint::GRAPHICS,
                                self.pipeline_layout,
                                0,
                                &[self.textures.get(&mesh.texture_id).unwrap().0],
                                &[],
                            );
                        }
                        TextureId::User(_) => {
                            let dsc_sets = DSC_SETS.lock().unwrap();
                            if let Some(dsc_set) = unsafe { dsc_sets.get(SLIDER) } {
                                self.dev.cmd_bind_descriptor_sets(
                                    image.command_buffer,
                                    vk::PipelineBindPoint::GRAPHICS,
                                    self.pipeline_layout,
                                    0,
                                    &[*dsc_set],
                                    &[],
                                );
                            } else {
                                return;
                            }
                        }
                    }

                    index_buf_ptr
                        .add(index_base)
                        .copy_from_nonoverlapping(mesh.indices.as_ptr(), mesh.indices.len());
                    vertex_buf_ptr
                        .add(vertex_base)
                        .copy_from_nonoverlapping(mesh.vertices.as_ptr(), mesh.vertices.len());

                    let clip_rect = primitive.clip_rect;
                    let min = clip_rect.min;
                    let min = Pos2 {
                        x: f32::clamp(min.x, 0.0, 1920.0),
                        y: f32::clamp(min.y, 0.0, 1080.0),
                    };
                    let max = clip_rect.max;
                    let max = Pos2 {
                        x: f32::clamp(max.x, min.x, 1920.0),
                        y: f32::clamp(max.y, min.y, 1080.0),
                    };
                    self.dev.cmd_set_scissor(image.command_buffer, 0, &[*vk::Rect2D::builder()
                        .offset(vk::Offset2D { x: min.x.round() as i32, y: min.y.round() as i32 })
                        .extent(vk::Extent2D {
                            width: (max.x.round() - min.x) as u32,
                            height: (max.y.round() - min.y) as u32,
                        })]);
                    self.dev.cmd_set_viewport(image.command_buffer, 0, &[*vk::Viewport::builder()
                        .width(1920.0)
                        .height(1080.0)
                        .max_depth(1.0)]);
                    self.dev.cmd_draw_indexed(
                        image.command_buffer,
                        mesh.indices.len() as _,
                        1,
                        index_base as _,
                        vertex_base as _,
                        0,
                    );
                    index_base += mesh.indices.len();
                    vertex_base += mesh.vertices.len();
                }
                Primitive::Callback(_) => unimplemented!(),
            }
        }
    }

    unsafe fn update_textures(
        &mut self,
        textures_delta: TexturesDelta,
        cmd_buf: vk::CommandBuffer,
    ) {
        for id in textures_delta.free {
            self.free_texture(id);
        }

        let mut dsc_sets = DSC_SETS.lock().unwrap();
        let views = ATLAS_VIEWS.lock().unwrap();
        if dsc_sets.len() < views.len() {
            for image_view in views.iter() {
                let dsc_set = self
                    .dev
                    .allocate_descriptor_sets(
                        &vk::DescriptorSetAllocateInfo::builder()
                            .descriptor_pool(self.descriptor_pool)
                            .set_layouts(&[self.descriptor_set_layout]),
                    )
                    .unwrap()[0];
                self.dev.update_descriptor_sets(
                    &[*vk::WriteDescriptorSet::builder()
                        .dst_set(dsc_set)
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(&[*vk::DescriptorImageInfo::builder()
                            .image_view(*image_view)
                            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                            .sampler(self.sampler)])],
                    &[],
                );
                dsc_sets.push(dsc_set);
            }
        }

        for (id, delta) in textures_delta.set {
            if delta.is_whole() {
                let image = Texture::new(&self.dev, &self.allocator, &delta.image);

                let dsc_set = self
                    .dev
                    .allocate_descriptor_sets(
                        &vk::DescriptorSetAllocateInfo::builder()
                            .descriptor_pool(self.descriptor_pool)
                            .set_layouts(&[self.descriptor_set_layout]),
                    )
                    .unwrap()[0];
                self.dev.update_descriptor_sets(
                    &[*vk::WriteDescriptorSet::builder()
                        .dst_set(dsc_set)
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(&[*vk::DescriptorImageInfo::builder()
                            .image_view(image.view)
                            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                            .sampler(self.sampler)])],
                    &[],
                );

                self.apply_image_delta(&image, &delta, cmd_buf);
                self.textures.insert(id, (dsc_set, image));
            } else {
                self.apply_image_delta(&self.textures[&id].1, &delta, cmd_buf);
            }
        }
    }

    unsafe fn apply_image_delta(
        &self,
        image: &Texture,
        delta: &epaint::ImageDelta,
        cmd_buf: vk::CommandBuffer,
    ) {
        let data = match &delta.image {
            ImageData::Color(image) => {
                image.pixels.iter().flat_map(|c| c.to_array()).collect::<Vec<_>>()
            }
            ImageData::Font(image) => {
                image.srgba_pixels(None).flat_map(|c| c.to_array()).collect::<Vec<_>>()
            }
        };

        let buf = Buffer::new(
            &self.allocator,
            &self.dev,
            data.len(),
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::SharingMode::default(),
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        let ptr = buf.mem.ptr;
        ptr.copy_from_nonoverlapping(data.as_ptr() as _, data.len());

        self.dev.cmd_copy_buffer_to_image(
            cmd_buf,
            buf.buf,
            image.image,
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

    unsafe fn free_texture(&mut self, id: TextureId) {
        if let Some((_, (_, image))) = self.textures.remove_entry(&id) {
            image.free(&self.dev)
        }
    }
}

#[derive(Clone)]
struct VulkanDataBuilder<M: EguiMenu> {
    instance: Option<ash::Instance>,
    physical_dev: Option<vk::PhysicalDevice>,
    dev: Option<ash::Device>,
    queue_family: Option<u32>,
    pipeline_cache: Option<vk::PipelineCache>,
    swapchain_data: Option<SwapchainData>,
    menu: Option<M>,
}

impl<M: EguiMenu> VulkanDataBuilder<M> {
    pub const fn new() -> Self {
        Self {
            instance: None,
            physical_dev: None,
            dev: None,
            queue_family: None,
            pipeline_cache: None,
            swapchain_data: None,
            menu: None,
        }
    }

    pub fn build(&mut self) -> VulkanData<M> {
        let instance = self.instance.take().expect("Vulkan instance was not initialized");
        let physical_dev =
            self.physical_dev.take().expect("Vulkan physical device was not initialized");
        let dev = self.dev.take().expect("Vulkan device was not initialized");
        let queue_family =
            self.queue_family.take().expect("Vulkan queue family was not initialized");
        let pipeline_cache =
            self.pipeline_cache.take().expect("Vulkan pipeline cache was not initialized");
        let swapchain_data =
            self.swapchain_data.take().expect("Vulkan swapchain data was not initialized");
        let menu = self.menu.take().expect("ImGui menu was not initialized");

        unsafe {
            let ctx = Context::default();
            // menu.init(&mut ctx, &mut dev);
            egui_backend_win32::init();

            let queue = dev.get_device_queue(queue_family, 0);

            let allocator = Allocator::new(&instance, physical_dev);

            let index_buf = Buffer::new(
                &allocator,
                &dev,
                1024 * 1024 * 4,
                vk::BufferUsageFlags::INDEX_BUFFER,
                vk::SharingMode::EXCLUSIVE,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );

            let vertex_buf = Buffer::new(
                &allocator,
                &dev,
                1024 * 1024 * 4,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::SharingMode::EXCLUSIVE,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );

            let descriptor_pool = dev
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

            let descriptor_set_layout = dev
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

            let pipeline_layout = dev
                .create_pipeline_layout(
                    &vk::PipelineLayoutCreateInfo::builder()
                        .set_layouts(&[descriptor_set_layout])
                        .push_constant_ranges(&[*vk::PushConstantRange::builder()
                            .stage_flags(vk::ShaderStageFlags::VERTEX)
                            .offset(0)
                            .size(mem::size_of::<f32>() as u32 * 2)]),
                    None,
                )
                .unwrap();

            let pipeline = create_pipeline(&dev, pipeline_layout, swapchain_data.render_pass);
            let sampler = dev
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

            VulkanData {
                instance,
                physical_dev,
                dev,
                queue_family,
                queue,
                pipeline_cache,
                descriptor_pool,
                swapchain_data,

                menu,

                ctx,
                allocator,
                index_buf,
                vertex_buf,

                textures: HashMap::new(),

                descriptor_set_layout,
                pipeline_layout,
                pipeline,
                sampler,
            }
        }
    }

    pub fn instance(&self) -> &ash::Instance {
        self.instance.as_ref().unwrap()
    }
}

fn create_pipeline(
    dev: &ash::Device,
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
    let vertex_shader_mod = unsafe { dev.create_shader_module(&info, None).unwrap() };

    let bytes_code = include_bytes!("shaders/frag.spv");
    let info = vk::ShaderModuleCreateInfo {
        code_size: bytes_code.len(),
        p_code: bytes_code.as_ptr() as _,
        ..Default::default()
    };
    let fragment_shader_mod = unsafe { dev.create_shader_module(&info, None).unwrap() };

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
    let rasterization_info = vk::PipelineRasterizationStateCreateInfo::builder().line_width(1.0);
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
        dev.create_graphics_pipelines(
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
                            .color_write_mask(
                                vk::ColorComponentFlags::R
                                    | vk::ColorComponentFlags::G
                                    | vk::ColorComponentFlags::B
                                    | vk::ColorComponentFlags::A,
                            )
                            .blend_enable(true)
                            .src_color_blend_factor(vk::BlendFactor::ONE)
                            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA),
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
        dev.destroy_shader_module(vertex_shader_mod, None);
        dev.destroy_shader_module(fragment_shader_mod, None);
    }
    pipeline
}

#[derive(Debug)]
struct Texture {
    image: vk::Image,
    view: vk::ImageView,
    buf: Allocation,
}

impl Texture {
    unsafe fn new(dev: &ash::Device, allocator: &Allocator, image_data: &ImageData) -> Texture {
        let extent = vk::Extent3D {
            width: image_data.width() as _,
            height: image_data.height() as _,
            depth: 1,
        };

        let image = dev
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

        let requirements = dev.get_image_memory_requirements(image);
        let buf = allocator.alloc(dev, requirements, vk::MemoryPropertyFlags::DEVICE_LOCAL);

        dev.bind_image_memory(image, buf.handle, 0).unwrap();

        let view = dev
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

        Self { image, view, buf }
    }

    unsafe fn free(self, dev: &ash::Device) {
        dev.destroy_image_view(self.view, None);
        dev.destroy_image(self.image, None);
        self.buf.free(dev);
    }
}

#[derive(Debug, Clone)]
struct SwapchainData {
    swapchain: vk::SwapchainKHR,
    render_pass: vk::RenderPass,
    command_pool: vk::CommandPool,
    images: Vec<SwapchainImageData>,
    extent: vk::Extent2D,
}

impl SwapchainData {
    pub unsafe fn new(
        swapchain: vk::SwapchainKHR,
        create_info: &vk::SwapchainCreateInfoKHR,
        instance: &ash::Instance,
        dev: &ash::Device,
        queue_family: u32,
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

        let render_pass = dev.create_render_pass(&render_pass, None).unwrap();

        let info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family);
        let command_pool = dev.create_command_pool(&info, None).unwrap();

        let swapchain_khr = ash::extensions::khr::Swapchain::new(instance, dev);

        let images = swapchain_khr
            .get_swapchain_images(swapchain)
            .unwrap()
            .into_iter()
            .map(|image| {
                SwapchainImageData::new(
                    dev,
                    image,
                    command_pool,
                    render_pass,
                    create_info.image_format,
                    extent,
                )
            })
            .collect();

        Self { swapchain, render_pass, command_pool, images, extent }
    }

    pub fn destroy(mut self, dev: &ash::Device) {
        unsafe {
            if self.render_pass != vk::RenderPass::null() {
                dev.destroy_render_pass(self.render_pass, None);
            }

            for i in self.images.drain(..) {
                i.destroy(dev, self.command_pool);
            }

            if self.command_pool != vk::CommandPool::null() {
                dev.destroy_command_pool(self.command_pool, None);
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SwapchainImageData {
    framebuffer: vk::Framebuffer,
    image_view: vk::ImageView,
    command_buffer: vk::CommandBuffer,
    fence: vk::Fence,
    semaphore: vk::Semaphore,
}

impl SwapchainImageData {
    pub unsafe fn new(
        dev: &ash::Device,
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
        let command_buffer = dev.allocate_command_buffers(&info).unwrap()[0];

        let info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
        let fence = dev.create_fence(&info, None).unwrap();

        let info = vk::SemaphoreCreateInfo::default();
        let semaphore = dev.create_semaphore(&info, None).unwrap();

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
        let image_view = dev.create_image_view(&info, None).unwrap();

        let attachments = [image_view];
        let info = vk::FramebufferCreateInfo::builder()
            .render_pass(render_pass)
            .attachments(&attachments)
            .width(extent.width)
            .height(extent.height)
            .layers(1);

        let framebuffer = dev.create_framebuffer(&info, None).unwrap();

        Self { framebuffer, image_view, command_buffer, fence, semaphore }
    }

    pub fn destroy(self, dev: &ash::Device, command_pool: vk::CommandPool) {
        unsafe {
            if self.framebuffer != vk::Framebuffer::null() {
                dev.destroy_framebuffer(self.framebuffer, None);
            }
            if self.image_view != vk::ImageView::null() {
                dev.destroy_image_view(self.image_view, None);
            }
            if self.command_buffer != vk::CommandBuffer::null() {
                dev.free_command_buffers(command_pool, &[self.command_buffer]);
            }
            if self.fence != vk::Fence::null() {
                dev.destroy_fence(self.fence, None);
            }
            if self.semaphore != vk::Semaphore::null() {
                dev.destroy_semaphore(self.semaphore, None);
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct Buffer {
    buf: vk::Buffer,
    mem: Allocation,
}

impl Buffer {
    pub fn new(
        allocator: &Allocator,
        dev: &ash::Device,
        size: usize,
        usage: vk::BufferUsageFlags,
        sharing_mode: vk::SharingMode,
        mem_props: vk::MemoryPropertyFlags,
    ) -> Self {
        let buffer_info =
            vk::BufferCreateInfo::builder().size(size as _).usage(usage).sharing_mode(sharing_mode);
        let buf = unsafe { dev.create_buffer(&buffer_info, None).unwrap() };

        let requirements = unsafe { dev.get_buffer_memory_requirements(buf) };

        let mem = allocator.alloc(dev, requirements, mem_props);

        unsafe { dev.bind_buffer_memory(buf, mem.handle, 0).unwrap() };

        Self { buf, mem }
    }
}

#[derive(Debug)]
pub(crate) struct Allocator {
    physical_dev_mem_props: vk::PhysicalDeviceMemoryProperties,
}

impl Allocator {
    pub fn new(instance: &ash::Instance, physical_dev: vk::PhysicalDevice) -> Self {
        let physical_dev_mem_props =
            unsafe { instance.get_physical_device_memory_properties(physical_dev) };
        Self { physical_dev_mem_props }
    }

    pub fn alloc(
        &self,
        dev: &ash::Device,
        requirements: vk::MemoryRequirements,
        memory_property_flags: vk::MemoryPropertyFlags,
    ) -> Allocation {
        let info = vk::MemoryAllocateInfo::builder()
            .allocation_size(requirements.size)
            .memory_type_index(self.mem_type(requirements, memory_property_flags).unwrap());
        let handle = unsafe { dev.allocate_memory(&info, None).unwrap() };
        let size = requirements.size;

        let is_mapped = !memory_property_flags.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL);
        let ptr = if is_mapped {
            unsafe { dev.map_memory(handle, 0, size, vk::MemoryMapFlags::empty()).unwrap() }
        } else {
            ptr::null()
        } as _;
        Allocation { handle, size, ptr, is_mapped }
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
pub(crate) struct Allocation {
    handle: vk::DeviceMemory,
    size: u64,
    ptr: *mut u8,
    is_mapped: bool,
}

impl Allocation {
    pub fn free(self, dev: &ash::Device) {
        unsafe {
            if self.is_mapped {
                dev.unmap_memory(self.handle)
            };
            dev.free_memory(self.handle, None);
        }
    }
}
