use std::{mem, ptr};

use ash::{vk, RawPtr};
use windows::{
    core::{s, w},
    Win32::{
        Foundation::{BOOL, HANDLE, HWND, LPARAM},
        System::{
            LibraryLoader::{GetProcAddress, LoadLibraryW},
            Threading::GetCurrentThread,
        },
        UI::{
            Input::KeyboardAndMouse::GetAsyncKeyState,
            WindowsAndMessaging::{
                EnumWindows, GetForegroundWindow, GetWindow, IsWindowVisible, GW_OWNER,
            },
        },
    },
};

use crate::{
    err,
    game_definitions::{GamePtr, OsiArgumentDesc, OsirisStaticGlobals},
    globals::Globals,
    info,
};

macro_rules! hook_definitions {
    {
        $($mod_name:ident($dll_name:literal) {
            $(
                $(#[symbol_name = $symbol_name:literal])?
                $(#[no_init = $init:ident])?
                fn $name:ident($($arg_name:ident: $arg:ty),* $(,)?) $(-> $ret: ty)? $body: block
            )*
        })*
    } => {
        pub(crate) fn init_hooks() -> anyhow::Result<()> {
            unsafe {
                $(
                    let $mod_name = LoadLibraryW(w!($dll_name))?;
                )*
                DetourTransactionBegin();
                DetourUpdateThread(GetCurrentThread());

                $($(
                    if_no_init_meta!(init_hook_from_name!($mod_name, $name $(, $symbol_name)?) $(, $init)?);
                )*)*

                DetourTransactionCommit();
                hook();
            }

            Ok(())
        }

        #[allow(non_snake_case, dead_code)]
        mod _hooks {
            use super::*;
            $($(
                pub extern "C" fn $name($($arg_name: $arg),*) $(-> $ret)? $body
            )*)*
        }

        pub(crate) use _hooks::*;

        #[allow(non_snake_case)]
        #[derive(Debug, Default)]
        pub(crate) struct Hooks {
            $($(
                pub $name: HookableFunction<extern "C" fn($($arg_name: $arg),*) $(-> $ret)?>,
            )*)*
        }

        impl Hooks {
            pub const fn new() -> Self {
                Self {
                    $($(
                        $name: HookableFunction::new(),
                    )*)*
                }
            }
        }

        #[allow(non_snake_case, dead_code)]
        mod original {
            use super::*;
            $($(
                pub extern "C" fn $name($($arg_name: $arg),*) $(-> $ret)? {
                    $crate::globals::Globals::hooks().$name.as_ref()($($arg_name),*)
                }
            )*)*
        }
    };
}

macro_rules! if_no_init_meta {
    ($expr:expr,yes) => {};
    ($expr:expr $(,)?) => {
        $expr
    };
}

macro_rules! init_hook {
    ($name:ident, $tgt:expr) => {
        Globals::hooks_mut().$name.set($tgt as _);
        DetourAttach(Globals::hooks_mut().$name.as_mut() as *mut _ as _, $name as _)
    };
}

macro_rules! init_hook_from_name {
    ($module:expr, $name:ident) => {
        init_hook_from_name!($module, $name, stringify!($name))
    };
    ($module:expr, $name:ident, $symbol_name:expr) => {{
        let Some(tgt) =
            GetProcAddress($module, windows::core::PCSTR(concat!($symbol_name, "\0").as_ptr()))
        else {
            anyhow::bail!(concat!("Failed to find ", $symbol_name));
        };
        init_hook!($name, tgt);
    }};
}

fn hook() {
    // unsafe {
    //     create_device_vk();
    //
    //     let ani = VK_DATA.instance.as_ref().unwrap().get_device_proc_addr(
    //         VK_DATA.fake_dev.as_ref().unwrap().handle(),
    //         c"vkAcquireNextImageKHR".as_ptr(),
    //     );
    //     let ani2 = VK_DATA.instance.as_ref().unwrap().get_device_proc_addr(
    //         VK_DATA.fake_dev.as_ref().unwrap().handle(),
    //         c"vkAcquireNextImage2KHR".as_ptr(),
    //     );
    //     let qp = VK_DATA.instance.as_ref().unwrap().get_device_proc_addr(
    //         VK_DATA.fake_dev.as_ref().unwrap().handle(),
    //         c"vkQueuePresentKHR".as_ptr(),
    //     );
    //     let cs = VK_DATA.instance.as_ref().unwrap().get_device_proc_addr(
    //         VK_DATA.fake_dev.as_ref().unwrap().handle(),
    //         c"vkCreateSwapchainKHR".as_ptr(),
    //     );
    //
    //     info!("{ani:?} {ani2:?} {qp:?} {cs:?}");
    //
    //     if let Some(dev) = VK_DATA.fake_dev.take() {
    //         dev.destroy_device(VK_DATA.allocator);
    //     }
    //
    //     if let Some(ani) = ani {
    //         VK_DATA.hwnd = Some(GetForegroundWindow());
    //
    //         DetourTransactionBegin();
    //         DetourUpdateThread(GetCurrentThread());
    //
    //         macro_rules! hook {
    //             ($name:ident, $tgt:expr) => {
    //                 Globals::hooks_mut().$name.set($tgt as _);
    //                 DetourAttach(Globals::hooks_mut().$name.as_mut() as *mut
    // _ as _, $name as _);             };
    //         }
    //
    //         hook!(vkAcquireNextImageKHR, ani);
    //         hook!(vkAcquireNextImage2KHR, ani2.unwrap());
    //         hook!(vkQueuePresentKHR, qp.unwrap());
    //         hook!(vkCreateSwapchainKHR, cs.unwrap());
    //
    //         DetourTransactionCommit();
    //     }
    // }
}

hook_definitions! {
sdl("SDL2.dll") {
    fn SDL_CreateWindow(
        title: *const u8,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        flags: u32,
    ) -> *const () {
        info!("SDL_CreateWindow");

        let res = original::SDL_CreateWindow(title, x, y, w, h, flags);

        unsafe {
            SDL_WINDOW = res;
        }

        res
    }

    fn SDL_PollEvent(event: *mut ()) -> i32 {
        info!("SDL_PollEvent");
        unsafe {
            let res = original::SDL_PollEvent(event);

            ImGui_ImplSDL2_ProcessEvent(event);

            res
        }
    }
}

vulkan("vulkan-1.dll") {
    fn vkCreateInstance(
        p_create_info: *const vk::InstanceCreateInfo,
        p_allocator: *const vk::AllocationCallbacks,
        p_instance: *mut vk::Instance,
    ) -> vk::Result {
        info!("vkCreateInstance");

        unsafe {
            let name = (*(*p_create_info).p_application_info).p_application_name;
            let engine_name = (*(*p_create_info).p_application_info).p_application_name;

            let name = std::ffi::CStr::from_ptr(name);
            let engine_name = std::ffi::CStr::from_ptr(engine_name);

            info!("{name:?} {engine_name:?}");
        }

        let res = original::vkCreateInstance(p_create_info, p_allocator, p_instance);

        unsafe {
            if INSTANCE == vk::Instance::null() {
                INSTANCE = *p_instance;
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
        info!("vkCreateDevice");

        let ret = original::vkCreateDevice(physical_device, p_create_info, p_allocator, p_device);

        unsafe {
            ALLOCATOR  = p_allocator.as_ref();
            DEV = *p_device;
            PHYSICAL_DEV = physical_device;
            init_vulkan();

            let create_swapchain = instance()
                .get_device_proc_addr(DEV, c"vkCreateSwapchainKHR".as_ptr())
                .unwrap();

            let submit = instance()
                .get_device_proc_addr(DEV, c"vkQueueSubmit2KHR".as_ptr())
                .unwrap();
            DetourTransactionBegin();
            DetourUpdateThread(GetCurrentThread());

            // init_hook!(vkQueueSubmit2KHR, submit);
            init_hook!(vkCreateSwapchainKHR, create_swapchain);

            DetourTransactionCommit();
        }

        ret
    }

    // #[no_init = yes]
    fn vkAcquireNextImageKHR(
        device: vk::Device,
        swapchain: vk::SwapchainKHR,
        timeout: u64,
        semaphore: vk::Semaphore,
        fence: vk::Fence,
        p_image_index: *mut u32,
    ) -> vk::Result {
        info!("vkAcquireNextImageKHR");
        let res = original::vkAcquireNextImageKHR(device, swapchain, timeout, semaphore, fence, p_image_index);

        unsafe {
            CUR_FRAME = *p_image_index;
        }

        res
    }

    // #[no_init = yes]
    fn vkAcquireNextImage2KHR(
        device: vk::Device,
        p_acquire_info: *const vk::AcquireNextImageInfoKHR,
        p_image_index: *mut u32,
    ) -> vk::Result {
        info!("vkAcquireNextImage2KHR");

        original::vkAcquireNextImage2KHR(device, p_acquire_info, p_image_index)
    }

    #[no_init = yes]
    fn vkCreateSwapchainKHR(
        device: vk::Device,
        p_create_info: *const vk::SwapchainCreateInfoKHR,
        p_allocator: *const vk::AllocationCallbacks,
        p_swapchain: *mut vk::SwapchainKHR,
    ) -> vk::Result {
        info!("vkCreateSwapchainKHR");
        cleanup_render_target();
        unsafe {
            IMAGE_EXTENT = (*p_create_info).image_extent;
        }

        original::vkCreateSwapchainKHR(device, p_create_info, p_allocator, p_swapchain)
    }

    // #[no_init = yes]
    // fn vkQueueSubmit2KHR(
    //     queue: vk::Queue,
    //     submit_count: u32,
    //     p_submits: *const vk::SubmitInfo2,
    //     fence: vk::Fence,
    // ) -> vk::Result {
    //     info!("vkQueueSubmit2KHR");
    //
    //     unsafe {
    //         let mut rendered = FRAME_RENDERED.lock().unwrap();
    //         if *rendered || SWAPCHAIN == vk::SwapchainKHR::null() {
    //             return original::vkQueueSubmit2KHR(queue, submit_count, p_submits, fence);
    //         }
    //         *rendered = true;
    //         drop(rendered);
    //
    //         let p_submits = p_submits as *mut vk::SubmitInfo2;
    //         render_im_gui_vulkan2(queue);
    //
    //         let fd = FRAMES[CUR_FRAME as usize];
    //         let info = vk::CommandBufferSubmitInfo {
    //             command_buffer: fd.command_buffer,
    //             ..Default::default()
    //         };
    //
    //         let infos = std::slice::from_raw_parts(
    //             (*p_submits).p_command_buffer_infos,
    //             (*p_submits).command_buffer_info_count as _,
    //         );
    //
    //         let new_infos = [infos, &[info]].concat();
    //
    //         (*p_submits).command_buffer_info_count += 1;
    //         (*p_submits).p_command_buffer_infos = new_infos.as_ptr();
    //
    //         // info!("vkQueueSubmit2KHR(");
    //         // info!("{queue:?}");
    //         // info!("{submit_count}");
    //         // info!("{:#?}", *p_submits);
    //         // if (*p_submits).command_buffer_info_count > 0 {
    //         //     info!("{:#?}", *(*p_submits).p_command_buffer_infos);
    //         // }
    //         // if (*p_submits).wait_semaphore_info_count > 0 {
    //         //     info!("{:#?}", *(*p_submits).p_wait_semaphore_infos);
    //         // }
    //         // if (*p_submits).signal_semaphore_info_count > 0 {
    //         //     info!("{:#?}", *(*p_submits).p_signal_semaphore_infos);
    //         // }
    //         // info!("{fence:?}");
    //         // info!(")");
    //
    //         original::vkQueueSubmit2KHR(queue, submit_count, p_submits, fence)
    //     }
    // }

    // #[no_init = yes]
    fn vkQueuePresentKHR(
        queue: vk::Queue,
        p_present_info: *const vk::PresentInfoKHR,
    ) -> vk::Result {
        info!("vkQueuePresentKHR");
        unsafe {
            render_im_gui_vulkan(queue, p_present_info);
        }

        original::vkQueuePresentKHR(queue, p_present_info)
    }

}

osiris("Osiris.dll") {
    #[symbol_name = "?RegisterDIVFunctions@COsiris@@QEAAXPEAUTOsirisInitFunction@@@Z"]
    fn RegisterDivFunctions(a: *const u8, b: *const u8) -> i32 {
        unsafe {
            let osiris_module = LoadLibraryW(w!("Osiris.dll")).unwrap();
            let osiris_ctor_proc =
                GetProcAddress(osiris_module, s!("??0COsiris@@QEAA@XZ")).unwrap() as _;

            Globals::osiris_globals_set(find_osiris_globals(osiris_ctor_proc));

            Globals::hooks_mut().Call.set(*(b as *const *const ()).add(1));
            Globals::hooks_mut().Query.set(*(b as *const *const ()).add(2));

            original::RegisterDivFunctions(a, b)
        }
    }

    #[no_init = yes]
    fn Call(handle: u32, params: GamePtr<OsiArgumentDesc>) -> bool {
        original::Call(handle, params)
    }

    #[no_init = yes]
    fn Query(handle: u32, params: GamePtr<OsiArgumentDesc>) -> bool {
        original::Query(handle, params)
    }
}
}

const MIN_IMAGE_COUNT: u32 = 2;

static mut SDL_WINDOW: *const () = ptr::null();

static mut FRAMES: [ImGui_ImplVulkanH_Frame; 4] = [ImGui_ImplVulkanH_Frame::new(); 4];
static mut FRAME_SEMAPHORES: [ImGui_ImplVulkanH_FrameSemaphores; 4] =
    [ImGui_ImplVulkanH_FrameSemaphores::new(); 4];
static mut CUR_FRAME: u32 = 0;

static mut ALLOCATOR: Option<&vk::AllocationCallbacks> = None;
static mut INSTANCE: vk::Instance = vk::Instance::null();
static mut DEV: vk::Device = vk::Device::null();
static mut PHYSICAL_DEV: vk::PhysicalDevice = vk::PhysicalDevice::null();
static mut QUEUE_FAMILY: u32 = u32::MAX - 1;
static mut DESCRIPTOR_POOL: vk::DescriptorPool = vk::DescriptorPool::null();
static mut RENDER_PASS: vk::RenderPass = vk::RenderPass::null();
static mut IMAGE_EXTENT: vk::Extent2D = vk::Extent2D { width: 1920, height: 1080 };

static mut INSTANCE_LOADED: Option<ash::Instance> = None;
static mut DEV_LOADED: Option<ash::Device> = None;

fn instance() -> &'static ash::Instance {
    unsafe { INSTANCE_LOADED.as_ref().unwrap() }
}

fn dev() -> &'static ash::Device {
    unsafe { DEV_LOADED.as_ref().unwrap() }
}

fn render_im_gui_vulkan(queue: vk::Queue, p_present_info: *const vk::PresentInfoKHR) {
    info!("render_im_gui_vulkan2");

    unsafe {
        let swapchain = *(*p_present_info).p_swapchains;
        if FRAMES[0].framebuffer == vk::Framebuffer::null() {
            create_render_target(swapchain);
        }

        let fd = &mut FRAMES[CUR_FRAME as usize];

        dev().wait_for_fences(&[fd.fence], true, u64::MAX).unwrap();
        dev().reset_fences(&[fd.fence]).unwrap();

        dev()
            .reset_command_buffer(fd.command_buffer, vk::CommandBufferResetFlags::empty())
            .unwrap();
        let mut info = vk::CommandBufferBeginInfo::default();
        info.flags |= vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT;
        dev().begin_command_buffer(fd.command_buffer, &info).unwrap();

        let info = vk::RenderPassBeginInfo {
            render_pass: RENDER_PASS,
            framebuffer: fd.framebuffer,
            render_area: vk::Rect2D { extent: IMAGE_EXTENT, ..Default::default() },
            ..Default::default()
        };
        dev().cmd_begin_render_pass(fd.command_buffer, &info, vk::SubpassContents::INLINE);

        if (*imgui::sys::igGetIO()).BackendRendererUserData.is_null() {
            let init_info = ImGui_ImplVulkan_InitInfo {
                instance: INSTANCE,
                physical_device: PHYSICAL_DEV,
                device: DEV,
                queue_family: QUEUE_FAMILY,
                queue,
                pipeline_cache: vk::PipelineCache::null(),
                descriptor_pool: DESCRIPTOR_POOL,
                min_image_count: MIN_IMAGE_COUNT,
                image_count: MIN_IMAGE_COUNT,
                MSAA_samples: vk::SampleCountFlags::TYPE_1,
                allocator: ALLOCATOR.as_raw_ptr(),
                ..ImGui_ImplVulkan_InitInfo::new()
            };
            ImGui_ImplVulkan_Init(&init_info, RENDER_PASS);
            ImGui_ImplVulkan_CreateFontsTexture(FRAMES[0].command_buffer);
        }

        ImGui_ImplVulkan_NewFrame();
        ImGui_ImplSDL2_NewFrame();

        imgui::sys::igNewFrame();
        imgui::sys::igShowDemoWindow(ptr::null_mut());
        imgui::sys::igRender();

        ImGui_ImplVulkan_RenderDrawData(
            imgui::sys::igGetDrawData(),
            fd.command_buffer,
            vk::Pipeline::null(),
        );

        dev().cmd_end_render_pass(fd.command_buffer);
        dev().end_command_buffer(fd.command_buffer).unwrap();

        let wait_semaphores_count = (*p_present_info).wait_semaphore_count;

        let cb =
            vk::CommandBufferSubmitInfo { command_buffer: fd.command_buffer, ..Default::default() };

        let ws = vk::SemaphoreSubmitInfo {
            stage_mask: vk::PipelineStageFlags2::FRAGMENT_SHADER,
            semaphore: *(*p_present_info).p_wait_semaphores,
            value: 1,
            ..Default::default()
        };

        let ss = vk::SemaphoreSubmitInfo {
            semaphore: FRAME_SEMAPHORES[CUR_FRAME as usize].image_acquired_semaphore,
            value: 1,
            ..Default::default()
        };

        let info = vk::SubmitInfo2 {
            command_buffer_info_count: 1,
            p_command_buffer_infos: &cb,

            // wait_semaphore_info_count: wait_semaphores_count,
            // p_wait_semaphore_infos: &ws,
            signal_semaphore_info_count: 1,
            p_signal_semaphore_infos: &ss,
            ..Default::default()
        };

        dev().queue_submit2(queue, &[info], fd.fence).unwrap();

        // let v = 0;
        // let info = vk::SemaphoreWaitInfo {
        //     semaphore_count: wait_semaphores_count,
        //     p_semaphores: (*p_present_info).p_wait_semaphores,
        //     p_values: &v,
        //     ..Default::default()
        // };
        // dev().wait_semaphores(&info, u64::MAX).unwrap();
        //
        // let stages_wait =
        //     vec![vk::PipelineStageFlags::FRAGMENT_SHADER;
        // wait_semaphores_count as usize];
        //
        // let info = vk::SubmitInfo {
        //     command_buffer_count: 1,
        //     p_command_buffers: &fd.command_buffer,
        //
        //     // p_wait_dst_stage_mask: stages_wait.as_ptr(),
        //     // wait_semaphore_count: wait_semaphores_count,
        //     // p_wait_semaphores: (*p_present_info).p_wait_semaphores,
        //     signal_semaphore_count: 1,
        //     p_signal_semaphores: &FRAME_SEMAPHORES[CUR_FRAME as
        // usize].image_acquired_semaphore,     ..Default::default()
        // };
        //
        // dev().queue_submit(queue, &[info], fd.fence).unwrap();
    }
}

fn init_vulkan() {
    info!("init_vulkan");
    unsafe {
        if imgui::sys::igGetCurrentContext().is_null() {
            imgui::sys::igCreateContext(std::ptr::null_mut());
            // ImGui_ImplWin32_Init(GetForegroundWindow().0 as _);
            // unsafe extern "system" fn is_main(handle: HWND, lparam: LPARAM) -> BOOL {
            //     if GetWindow(handle, GW_OWNER) == HWND::default()
            //         && IsWindowVisible(handle).as_bool()
            //     {
            //         *(lparam.0 as *mut HWND) = handle;
            //         false.into()
            //     } else {
            //         true.into()
            //     }
            // }
            //
            // let mut hwnd = HWND(0);
            // let _ = EnumWindows(Some(is_main), LPARAM(&mut hwnd as *mut _ as _));
            // info!("{hwnd:?} {:?}", GetForegroundWindow());
            // ImGui_ImplWin32_Init(hwnd.0 as _);
            // ImGui_ImplWin32_EnableDpiAwareness();
            ImGui_ImplSDL2_InitForVulkan(SDL_WINDOW);
            let io = imgui::sys::igGetIO();
            (*io).IniFilename = ptr::null();
            (*io).LogFilename = ptr::null();
            (*io).ConfigFlags |= imgui::sys::ImGuiConfigFlags_NavEnableKeyboard as i32;
            (*io).ConfigFlags |= imgui::sys::ImGuiConfigFlags_NavEnableGamepad as i32;
        }

        INSTANCE_LOADED = Some(ash::Instance::load(ash::Entry::linked().static_fn(), INSTANCE));
        DEV_LOADED = Some(ash::Device::load(instance().fp_v1_0(), DEV));

        QUEUE_FAMILY = instance()
            .get_physical_device_queue_family_properties(PHYSICAL_DEV)
            .into_iter()
            .position(|family| family.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .unwrap() as _;

        let pool_sizes = [
            vk::DescriptorPoolSize { ty: vk::DescriptorType::SAMPLER, descriptor_count: 1000 },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::SAMPLED_IMAGE,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_TEXEL_BUFFER,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_TEXEL_BUFFER,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER_DYNAMIC,
                descriptor_count: 1000,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::INPUT_ATTACHMENT,
                descriptor_count: 1000,
            },
        ];

        let pool_info = vk::DescriptorPoolCreateInfo {
            flags: vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET,
            max_sets: 1000 * pool_sizes.len() as u32,
            pool_size_count: pool_sizes.len() as _,
            p_pool_sizes: pool_sizes.as_ptr(),
            ..Default::default()
        };
        DESCRIPTOR_POOL = dev().create_descriptor_pool(&pool_info, ALLOCATOR).unwrap();

        let attachment = vk::AttachmentDescription {
            format: vk::Format::B8G8R8A8_UNORM,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::LOAD,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            ..Default::default()
        };

        let color_attachment = vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        };

        let subpass = vk::SubpassDescription {
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            color_attachment_count: 1,
            p_color_attachments: &color_attachment,
            ..Default::default()
        };

        let dependency = vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: vk::AccessFlags::empty(),
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            ..Default::default()
        };

        let info = vk::RenderPassCreateInfo {
            attachment_count: 1,
            p_attachments: &attachment,
            subpass_count: 1,
            p_subpasses: &subpass,
            dependency_count: 1,
            p_dependencies: &dependency,
            ..Default::default()
        };

        RENDER_PASS = dev().create_render_pass(&info, ALLOCATOR).unwrap();
    }
}

fn create_render_target(swapchain: vk::SwapchainKHR) {
    info!("create_render_target2");
    unsafe {
        let swapchain_ext = ash::extensions::khr::Swapchain::new(instance(), dev());
        let backbuffers = swapchain_ext.get_swapchain_images(swapchain).unwrap();

        for (i, image) in backbuffers.iter().enumerate() {
            let fd = &mut FRAMES[i];

            fd.backbuffer = *image;

            let info = vk::CommandPoolCreateInfo {
                flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                queue_family_index: QUEUE_FAMILY,
                ..Default::default()
            };
            fd.command_pool = dev().create_command_pool(&info, ALLOCATOR).unwrap();

            let info = vk::CommandBufferAllocateInfo {
                level: vk::CommandBufferLevel::PRIMARY,
                command_pool: fd.command_pool,
                command_buffer_count: 1,
                ..Default::default()
            };
            fd.command_buffer = dev().allocate_command_buffers(&info).unwrap()[0];

            let info =
                vk::FenceCreateInfo { flags: vk::FenceCreateFlags::SIGNALED, ..Default::default() };
            fd.fence = dev().create_fence(&info, ALLOCATOR).unwrap();

            let info = vk::SemaphoreCreateInfo::default();
            FRAME_SEMAPHORES[i].image_acquired_semaphore =
                dev().create_semaphore(&info, ALLOCATOR).unwrap();
            FRAME_SEMAPHORES[i].render_complete_semaphore =
                dev().create_semaphore(&info, ALLOCATOR).unwrap();
        }

        let mut info = vk::ImageViewCreateInfo {
            view_type: vk::ImageViewType::TYPE_2D,
            format: vk::Format::B8G8R8A8_UNORM,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };

        for fd in FRAMES.iter_mut().take(backbuffers.len()) {
            info.image = fd.backbuffer;
            fd.backbuffer_view = dev().create_image_view(&info, ALLOCATOR).unwrap();
        }

        let mut info = vk::FramebufferCreateInfo {
            render_pass: RENDER_PASS,
            attachment_count: 1,
            width: IMAGE_EXTENT.width,
            height: IMAGE_EXTENT.height,
            layers: 1,
            ..Default::default()
        };

        for fd in FRAMES.iter_mut().take(backbuffers.len()) {
            info.p_attachments = &fd.backbuffer_view;
            fd.framebuffer = dev().create_framebuffer(&info, ALLOCATOR).unwrap();
        }
    }
}

fn cleanup_render_target() {
    info!("cleanup_render_target2");
    unsafe {
        for frame in FRAMES.iter_mut() {
            if frame.fence != vk::Fence::null() {
                dev().destroy_fence(frame.fence, ALLOCATOR);
                frame.fence = vk::Fence::null();
            }
            if frame.command_buffer != vk::CommandBuffer::null() {
                dev().free_command_buffers(frame.command_pool, &[frame.command_buffer]);
                frame.command_buffer = vk::CommandBuffer::null();
            }
            if frame.command_pool != vk::CommandPool::null() {
                dev().destroy_command_pool(frame.command_pool, ALLOCATOR);
                frame.command_pool = vk::CommandPool::null();
            }
            if frame.backbuffer_view != vk::ImageView::null() {
                dev().destroy_image_view(frame.backbuffer_view, ALLOCATOR);
                frame.backbuffer_view = vk::ImageView::null();
            }
            if frame.framebuffer != vk::Framebuffer::null() {
                dev().destroy_framebuffer(frame.framebuffer, ALLOCATOR);
                frame.framebuffer = vk::Framebuffer::null();
            }
        }

        for semaphores in FRAME_SEMAPHORES.iter_mut() {
            if semaphores.image_acquired_semaphore != vk::Semaphore::null() {
                dev().destroy_semaphore(semaphores.image_acquired_semaphore, ALLOCATOR);
                semaphores.image_acquired_semaphore = vk::Semaphore::null();
            }
            if semaphores.render_complete_semaphore != vk::Semaphore::null() {
                dev().destroy_semaphore(semaphores.render_complete_semaphore, ALLOCATOR);
                semaphores.render_complete_semaphore = vk::Semaphore::null();
            }
        }
    }
}

#[link(name = "detours", kind = "static")]
extern "system" {
    fn DetourTransactionBegin();
    fn DetourUpdateThread(handle: HANDLE);
    fn DetourAttach(
        ppPointer: *mut *const libc::c_void,
        pDetour: *const libc::c_void,
    ) -> libc::c_long;
    fn DetourTransactionCommit();
}

#[link(name = "imgui_backends", kind = "static")]
extern "C" {
    fn ImGui_ImplSDL2_InitForVulkan(window: *const ()) -> bool;
    fn ImGui_ImplSDL2_NewFrame();
    fn ImGui_ImplSDL2_ProcessEvent(event: *const ()) -> bool;
    fn ImGui_ImplWin32_Init(hwnd: *mut libc::c_void) -> bool;
    fn ImGui_ImplWin32_NewFrame();
    fn ImGui_ImplWin32_EnableDpiAwareness();
    fn ImGui_ImplVulkan_Init(info: *const ImGui_ImplVulkan_InitInfo, render_pass: vk::RenderPass);
    fn ImGui_ImplVulkan_NewFrame();
    fn ImGui_ImplVulkan_CreateFontsTexture(command_buffer: vk::CommandBuffer) -> bool;
    fn ImGui_ImplVulkan_RenderDrawData(
        draw_data: *const imgui::sys::ImDrawData,
        command: vk::CommandBuffer,
        pipeline: vk::Pipeline,
    );
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
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

#[repr(C)]
#[derive(Clone, Copy, Debug)]
#[allow(non_camel_case_types)]
struct ImGui_ImplVulkanH_Frame {
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    fence: vk::Fence,
    backbuffer: vk::Image,
    backbuffer_view: vk::ImageView,
    framebuffer: vk::Framebuffer,
}

impl ImGui_ImplVulkanH_Frame {
    pub const fn new() -> Self {
        Self {
            command_pool: vk::CommandPool::null(),
            command_buffer: vk::CommandBuffer::null(),
            fence: vk::Fence::null(),
            backbuffer: vk::Image::null(),
            backbuffer_view: vk::ImageView::null(),
            framebuffer: vk::Framebuffer::null(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
#[allow(non_camel_case_types)]
struct ImGui_ImplVulkanH_FrameSemaphores {
    image_acquired_semaphore: vk::Semaphore,
    render_complete_semaphore: vk::Semaphore,
}

impl ImGui_ImplVulkanH_FrameSemaphores {
    pub const fn new() -> Self {
        Self {
            image_acquired_semaphore: vk::Semaphore::null(),
            render_complete_semaphore: vk::Semaphore::null(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct HookableFunction<T> {
    ptr: Option<T>,
}

impl<T> Default for HookableFunction<T> {
    fn default() -> Self {
        Self { ptr: Default::default() }
    }
}

impl<T> AsRef<T> for HookableFunction<T> {
    fn as_ref(&self) -> &T {
        match &self.ptr {
            None => panic!("function not initialized"),
            Some(ptr) => ptr,
        }
    }
}

impl<T> AsMut<T> for HookableFunction<T> {
    fn as_mut(&mut self) -> &mut T {
        match &mut self.ptr {
            None => panic!("function not initialized"),
            Some(ptr) => ptr,
        }
    }
}

impl<T> HookableFunction<T> {
    pub const fn new() -> Self {
        Self { ptr: None }
    }

    pub fn set(&mut self, ptr: *const ()) {
        self.ptr = Some(unsafe { mem::transmute_copy(&ptr) });
    }
}

unsafe fn find_osiris_globals(ctor_proc: *const u8) -> Option<OsirisStaticGlobals> {
    let addr = resolve_real_function_address(ctor_proc);

    let mut globals = [ptr::null::<()>(); 9];
    let mut found_globals = 0;

    for i in 0..0x500 {
        let ptr = addr.add(i);

        if (*ptr == 0x49 || *ptr == 0x48)
            && *ptr.add(1) == 0x8B
            && *ptr.add(3) == 0x48
            && *ptr.add(4) == 0x89
            && (*ptr.add(5) & 0xC7) == 0x05
        {
            let rel_offset = *(ptr.add(6) as *const i32) as isize;
            let osi_ptr = ptr.offset(rel_offset + 10);
            globals[found_globals] = osi_ptr as _;
            found_globals += 1;
            if found_globals == 9 {
                break;
            }
        }
    }

    if found_globals < 9 {
        err!("Could not locate global Osiris variables");
        return None;
    }

    let osiris_globals = OsirisStaticGlobals {
        variables: globals[0],
        types: globals[1],
        enums: globals[2],
        functions: GamePtr::new(globals[3] as _),
        objects: globals[4],
        goals: globals[5],
        adapters: globals[6],
        databases: globals[7],
        nodes: globals[8],
    };

    Some(osiris_globals)
}

unsafe fn resolve_real_function_address(ptr: *const u8) -> *const u8 {
    if *ptr == 0xE9 {
        let rel_offset = *(ptr.add(1) as *const i32) as isize;
        return ptr.offset(rel_offset + 5) as _;
    }

    for i in 0..64 {
        let p = ptr.add(i);
        if *p == 0x48
            && *p.add(1) == 0x83
            && *p.add(2) == 0x3D
            && *p.add(6) == 0x00
            && *p.add(13) == 0xE9
        {
            let rel_offset = *(p.add(14) as *const i32) as isize;
            return p.offset(18 + rel_offset) as _;
        }
    }

    ptr
}
