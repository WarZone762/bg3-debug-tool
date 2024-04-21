use std::{mem, ops::DerefMut, ptr, sync::Mutex};

use ash::vk;
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, LRESULT, WPARAM},
    System::Threading::GetCurrentThread,
    UI::{
        Shell::{DefSubclassProc, SetWindowSubclass},
        WindowsAndMessaging::{EnumWindows, GetWindow, IsWindowVisible, GW_OWNER},
    },
};

use crate::{
    hook_definitions,
    hooks::{DetourTransactionBegin, DetourTransactionCommit, DetourUpdateThread},
};

pub(crate) fn init(menu: impl ImGuiMenu<ash::Device> + 'static) -> anyhow::Result<()> {
    unsafe {
        DATA_BUILDER.lock().unwrap().menu = Some(Box::new(menu));
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
            ash::Instance::load(ash::Entry::linked().static_fn(), *p_instance)
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

                // TODO
                // let create_pipeline_cache = vk_data
                //     .instance
                //     .get_device_proc_addr(*p_device, c"vkCreatePipelineCache".as_ptr())
                //     .unwrap();
                // let create_swapchain = vk_data
                //     .instance
                //     .get_device_proc_addr(*p_device, c"vkCreateSwapchainKHR".as_ptr())
                //     .unwrap();
                // let queue_present = vk_data
                //     .instance
                //     .get_device_proc_addr(*p_device, c"vkQueuePresentKHR".as_ptr())
                //     .unwrap();
                //
                // DetourTransactionBegin();
                // DetourUpdateThread(GetCurrentThread());
                //
                // HOOKS.vkCreatePipelineCache.attach(create_pipeline_cache as _);
                // HOOKS.vkCreateSwapchainKHR.attach(create_swapchain as _);
                // HOOKS.vkQueuePresentKHR.attach(queue_present as _);
                //
                // DetourTransactionCommit();
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

                DetourTransactionBegin();
                DetourUpdateThread(GetCurrentThread());

                HOOKS.vkCreatePipelineCache.attach(create_pipeline_cache as _);
                HOOKS.vkCreateSwapchainKHR.attach(create_swapchain as _);
                HOOKS.vkQueuePresentKHR.attach(queue_present as _);

                DetourTransactionCommit();
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
        let res = original::vkCreateSwapchainKHR(device, p_create_info, p_allocator, p_swapchain);

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
            DATA
                .lock()
                .unwrap()
                .as_mut()
                .unwrap()
                .present(
                    (p_present_info as *mut vk::PresentInfoKHR).as_mut().unwrap()
                );
        }

        original::vkQueuePresentKHR(queue, p_present_info)
    }

}
}

pub(crate) struct VulkanData<M: ImGuiMenu<ash::Device>> {
    instance: ash::Instance,
    physical_dev: vk::PhysicalDevice,
    dev: ash::Device,
    queue_family: u32,
    queue: vk::Queue,
    pipeline_cache: vk::PipelineCache,
    descriptor_pool: vk::DescriptorPool,
    swapchain_data: SwapchainData,

    ctx: imgui::Context,
    imgui_init: bool,
    menu: M,
}

impl<M: ImGuiMenu<ash::Device>> VulkanData<M> {
    fn present(&mut self, present_info: &mut vk::PresentInfoKHR) {
        unsafe {
            if present_info.swapchain_count != 1
                || *present_info.p_swapchains != self.swapchain_data.swapchain
            {
                return;
            }

            let image = &self.swapchain_data.images[*present_info.p_image_indices as usize];
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

            let clear_values = [vk::ClearValue::default()];
            let info = vk::RenderPassBeginInfo::builder()
                .render_pass(self.swapchain_data.render_pass)
                .framebuffer(image.framebuffer)
                .render_area(*vk::Rect2D::builder().extent(self.swapchain_data.extent))
                .clear_values(&clear_values);
            self.dev.cmd_begin_render_pass(
                image.command_buffer,
                &info,
                vk::SubpassContents::INLINE,
            );

            if !self.imgui_init {
                let info = ImGui_ImplVulkan_InitInfo {
                    instance: self.instance.handle(),
                    physical_device: self.physical_dev,
                    device: self.dev.handle(),
                    queue_family: self.queue_family,
                    queue: self.queue,
                    pipeline_cache: self.pipeline_cache,
                    descriptor_pool: self.descriptor_pool,
                    min_image_count: self.swapchain_data.images.len() as _,
                    image_count: self.swapchain_data.images.len() as _,
                    MSAA_samples: vk::SampleCountFlags::TYPE_1,
                    ..ImGui_ImplVulkan_InitInfo::new()
                };
                ImGui_ImplVulkan_Init(&info, self.swapchain_data.render_pass);
                ImGui_ImplVulkan_CreateFontsTexture(self.swapchain_data.images[0].command_buffer);

                self.imgui_init = true;
            }

            ImGui_ImplVulkan_NewFrame();
            ImGui_ImplWin32_NewFrame();

            self.menu.pre_render(&mut self.ctx);
            let ui = self.ctx.new_frame();
            self.menu.render(ui);
            self.ctx.render();

            ImGui_ImplVulkan_RenderDrawData(
                self.ctx.render(),
                image.command_buffer,
                vk::Pipeline::null(),
            );

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
}

#[derive(Clone)]
struct VulkanDataBuilder<M: ImGuiMenu<ash::Device>> {
    instance: Option<ash::Instance>,
    physical_dev: Option<vk::PhysicalDevice>,
    dev: Option<ash::Device>,
    queue_family: Option<u32>,
    pipeline_cache: Option<vk::PipelineCache>,
    swapchain_data: Option<SwapchainData>,
    menu: Option<M>,
}

impl<M: ImGuiMenu<ash::Device>> VulkanDataBuilder<M> {
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
        let mut dev = self.dev.take().expect("Vulkan device was not initialized");
        let queue_family =
            self.queue_family.take().expect("Vulkan queue family was not initialized");
        let pipeline_cache =
            self.pipeline_cache.take().expect("Vulkan pipeline cache was not initialized");
        let swapchain_data =
            self.swapchain_data.take().expect("Vulkan swapchain data was not initialized");
        let mut menu = self.menu.take().expect("ImGui menu was not initialized");

        unsafe {
            let mut ctx = imgui::Context::create();
            menu.initialize(&mut ctx, &mut dev);

            unsafe extern "system" fn is_main(handle: HWND, lparam: LPARAM) -> BOOL {
                if GetWindow(handle, GW_OWNER) == HWND::default()
                    && IsWindowVisible(handle).as_bool()
                {
                    *(lparam.0 as *mut HWND) = handle;
                    false.into()
                } else {
                    true.into()
                }
            }

            let mut hwnd = HWND(0);
            let _ = EnumWindows(Some(is_main), LPARAM(&mut hwnd as *mut _ as _));

            ImGui_ImplWin32_Init(hwnd.0 as _);

            let queue = dev.get_device_queue(queue_family, 0);

            let pool_sizes = [*vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(10)];

            let info = vk::DescriptorPoolCreateInfo::builder()
                .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
                .max_sets(1)
                .pool_sizes(&pool_sizes);
            let descriptor_pool = dev.create_descriptor_pool(&info, None).unwrap();

            unsafe extern "system" fn subclass_wnd_proc(
                hwnd: HWND,
                umsg: u32,
                wparam: WPARAM,
                lparam: LPARAM,
                _uid_subclass: usize,
                _dwref_data: usize,
            ) -> LRESULT {
                ImGui_ImplWin32_WndProcHandler(hwnd, umsg, wparam, lparam);

                let data = DATA.get_mut().unwrap();
                let io = data.as_ref().unwrap().ctx.io();
                if io.want_capture_mouse || io.want_capture_keyboard {
                    return LRESULT(1);
                }

                DefSubclassProc(hwnd, umsg, wparam, lparam)
            }

            SetWindowSubclass(hwnd, Some(subclass_wnd_proc), 1, 0);

            VulkanData {
                instance,
                physical_dev,
                dev,
                queue_family,
                queue,
                pipeline_cache,
                descriptor_pool,
                swapchain_data,

                ctx,
                imgui_init: false,
                menu,
            }
        }
    }

    pub fn instance(&self) -> &ash::Instance {
        self.instance.as_ref().unwrap()
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
            .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let color_attachment =
            vk::AttachmentReference::builder().layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
        let color_attachments = [*color_attachment];

        let subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachments);

        let attachments = [*attachment];
        let subpasses = [*subpass];
        let render_pass =
            vk::RenderPassCreateInfo::builder().attachments(&attachments).subpasses(&subpasses);

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

#[derive(Debug, Clone)]
struct SwapchainImageData {
    image: vk::Image,
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
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
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

        Self { image, framebuffer, image_view, command_buffer, fence, semaphore }
    }

    pub fn destroy(self, dev: &ash::Device, command_pool: vk::CommandPool) {
        unsafe {
            // if self.image != vk::Image::null() {
            //     dev.destroy_image(self.image, None);
            // }
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

static mut DATA: Mutex<Option<VulkanData<Box<dyn ImGuiMenu<ash::Device>>>>> = Mutex::new(None);
static mut DATA_BUILDER: Mutex<VulkanDataBuilder<Box<dyn ImGuiMenu<ash::Device>>>> =
    Mutex::new(VulkanDataBuilder::new());

#[link(name = "imgui_backends", kind = "static")]
extern "C" {
    fn ImGui_ImplWin32_Init(hwnd: *mut libc::c_void) -> bool;
    fn ImGui_ImplWin32_WndProcHandler(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM)
        -> bool;
    fn ImGui_ImplWin32_NewFrame();
    fn ImGui_ImplVulkan_Init(info: *const ImGui_ImplVulkan_InitInfo, render_pass: vk::RenderPass);
    fn ImGui_ImplVulkan_NewFrame();
    fn ImGui_ImplVulkan_CreateFontsTexture(command_buffer: vk::CommandBuffer) -> bool;
    fn ImGui_ImplVulkan_RenderDrawData(
        draw_data: *const imgui::DrawData,
        command: vk::CommandBuffer,
        pipeline: vk::Pipeline,
    );
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types, non_snake_case)]
struct ImGui_ImplVulkan_InitInfo {
    instance: vk::Instance,
    physical_device: vk::PhysicalDevice,
    device: vk::Device,
    queue_family: u32,
    queue: vk::Queue,
    pipeline_cache: vk::PipelineCache,
    descriptor_pool: vk::DescriptorPool,
    subpass: u32,
    min_image_count: u32,
    image_count: u32,
    MSAA_samples: vk::SampleCountFlags,
    allocator: *const vk::AllocationCallbacks,
    check_vk_result_fn: Option<extern "C" fn(err: vk::Result)>,
}

impl ImGui_ImplVulkan_InitInfo {
    pub const fn new() -> Self {
        Self {
            instance: vk::Instance::null(),
            physical_device: vk::PhysicalDevice::null(),
            device: vk::Device::null(),
            queue_family: 0,
            queue: vk::Queue::null(),
            pipeline_cache: vk::PipelineCache::null(),
            descriptor_pool: vk::DescriptorPool::null(),
            subpass: 0,
            min_image_count: 0,
            image_count: 0,
            MSAA_samples: vk::SampleCountFlags::empty(),
            allocator: ptr::null(),
            check_vk_result_fn: None,
        }
    }
}

pub(crate) trait ImGuiMenu<InitParam> {
    fn initialize(&mut self, _ctx: &mut imgui::Context, _params: &mut InitParam) {}
    fn pre_render(&mut self, _ctx: &mut imgui::Context) {}
    fn render(&mut self, ui: &mut imgui::Ui);
}

impl<M: ImGuiMenu<InitParam> + ?Sized, InitParam> ImGuiMenu<InitParam> for Box<M> {
    fn initialize(&mut self, ctx: &mut imgui::Context, params: &mut InitParam) {
        Box::deref_mut(self).initialize(ctx, params);
    }

    fn pre_render(&mut self, ctx: &mut imgui::Context) {
        Box::deref_mut(self).pre_render(ctx);
    }

    fn render(&mut self, ui: &mut imgui::Ui) {
        Box::deref_mut(self).render(ui);
    }
}
