use std::sync::Mutex;

use windows::{
    core::{IUnknown, Interface, GUID, HRESULT},
    Win32::{
        Foundation::HWND,
        Graphics::{
            Direct3D11::{ID3D11Device, ID3D11DeviceContext},
            Dxgi::{
                IDXGIFactory1, IDXGIFactory2, IDXGIOutput, IDXGISwapChain, IDXGISwapChain1,
                DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_CHAIN_FULLSCREEN_DESC,
            },
        },
    },
};

use crate::{
    hook_definitions,
    hooks::detour,
    menu::{backend, ImGuiMenu},
};

static mut DATA: Mutex<Option<DX11Data<Box<dyn ImGuiMenu<()>>>>> = Mutex::new(None);
static mut DATA_BUILDER: Mutex<DX11DataBuilder<Box<dyn ImGuiMenu<()>>>> =
    Mutex::new(DX11DataBuilder::new());

pub(crate) fn init(menu: impl ImGuiMenu<()> + 'static) -> anyhow::Result<()> {
    unsafe {
        DATA_BUILDER.lock().unwrap().menu = Some(Box::new(menu));
    }

    d3d11::init()?;
    hook()
}

mod d3d11 {
    use windows::Win32::{
        Foundation::HMODULE,
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE, D3D_FEATURE_LEVEL},
            Direct3D11::{ID3D11Device, ID3D11DeviceContext},
            Dxgi::IDXGIAdapter,
        },
    };

    use super::{DATA, DATA_BUILDER};
    use crate::hook_definitions;

    pub(crate) fn init() -> anyhow::Result<()> {
        hook()
    }

    hook_definitions! {
    d3d11("d3d11.dll") {
        fn D3D11CreateDevice(
            p_adapter: *mut IDXGIAdapter,
            driver_type: D3D_DRIVER_TYPE,
            software: HMODULE,
            flags: u32,
            p_feature_levels: *const D3D_FEATURE_LEVEL,
            feature_levels: u32,
            sdk_version: u32,
            pp_device: *mut ID3D11Device,
            p_feature_level: *mut D3D_FEATURE_LEVEL,
            pp_immediate_context: *mut ID3D11DeviceContext,
        ) -> windows::core::HRESULT {
            let res = original::D3D11CreateDevice(
                p_adapter,
                driver_type,
                software,
                flags,
                p_feature_levels,
                feature_levels,
                sdk_version,
                pp_device,
                p_feature_level,
                pp_immediate_context
            );

            unsafe {
                let mut data = DATA.lock().unwrap();
                if let Some(data) = data.as_mut() {
                    data.dev = (*pp_device).clone();
                } else {
                    let mut builder = DATA_BUILDER.lock().unwrap();
                    builder.dev = Some((*pp_device).clone());
                    *data = Some(builder.build());
                }
            }

            res
        }
    }
    }
}

hook_definitions! {
dxgi("dxgi.dll") {
    fn CreateDXGIFactory1(
        riid: *const GUID,
        pp_factory: *mut *mut libc::c_void,
    ) -> HRESULT {
        let res = original::CreateDXGIFactory1(riid, pp_factory);

        if res.is_err() {
            return res;
        }

        unsafe {
            if *riid != IDXGIFactory1::IID {
                return res;
            }

            if HOOKS.DXGICreateSwapChainForHwnd.is_attached() {
                return res;
            }

            let create_swapchain = (*(pp_factory as *mut IDXGIFactory2)).vtable().CreateSwapChainForHwnd;
            detour(|| {
                HOOKS.DXGICreateSwapChainForHwnd.attach(create_swapchain as _);
            });
        }

        res
    }

    #[no_init = yes]
    fn DXGICreateSwapChainForHwnd(
        this: *mut IDXGIFactory2,
        p_device: *mut IUnknown,
        hwnd: HWND,
        p_desc: *const DXGI_SWAP_CHAIN_DESC1,
        p_fullscreen: *const DXGI_SWAP_CHAIN_FULLSCREEN_DESC,
        p_restrict_to_output: *mut IDXGIOutput,
        pp_swapchain: *mut IDXGISwapChain1,
    ) -> HRESULT {
        let res = original::DXGICreateSwapChainForHwnd(
            this,
            p_device,
            hwnd,
            p_desc,
            p_fullscreen,
            p_restrict_to_output,
            pp_swapchain
        );

        unsafe {
            let swapchain_present = (*pp_swapchain).vtable().base__.Present;
            detour(|| {
                HOOKS.DXGISwapChainPresent.attach(swapchain_present as _);
            });
        }

        res
    }

    #[no_init = yes]
    fn DXGISwapChainPresent(
        this: IDXGISwapChain,
        sync_interval: u32,
        flags: u32,
    ) -> HRESULT {
        unsafe {
            if let Some(data) = &mut *DATA.lock().unwrap() {
                data.render();
            }
        };

        original::DXGISwapChainPresent(this, sync_interval, flags)
    }
}
}

#[derive(Debug)]
pub(crate) struct DX11Data<M: ImGuiMenu<()>> {
    dev: ID3D11Device,

    ctx: imgui::Context,
    menu: M,
}

impl<M: ImGuiMenu<()>> DX11Data<M> {
    fn render(&mut self) {
        unsafe {
            ImGui_ImplDX11_NewFrame();
            backend::new_frame();
            self.menu.pre_render(&mut self.ctx);
            let ui = self.ctx.new_frame();
            self.menu.render(ui);
            self.ctx.render();

            ImGui_ImplDX11_RenderDrawData(self.ctx.render());
        }
    }
}

struct DX11DataBuilder<M: ImGuiMenu<()>> {
    dev: Option<ID3D11Device>,
    menu: Option<M>,
}

impl<M: ImGuiMenu<()>> DX11DataBuilder<M> {
    const fn new() -> Self {
        Self { dev: None, menu: None }
    }

    fn build(&mut self) -> DX11Data<M> {
        let dev = self.dev.take().expect("DirectX 11 device menu was not initialized");
        let mut menu = self.menu.take().expect("ImGui menu was not initialized");

        let mut ctx = imgui::Context::create();
        menu.init(&mut ctx, &mut ());

        backend::init();

        unsafe {
            let dev_ctx = dev.GetImmediateContext().unwrap();
            ImGui_ImplDX11_Init(dev.clone(), dev_ctx);
        }

        DX11Data { dev, ctx, menu }
    }
}

#[link(name = "imgui_backends", kind = "static")]
extern "C" {
    fn ImGui_ImplDX11_Init(device: ID3D11Device, device_context: ID3D11DeviceContext) -> bool;
    fn ImGui_ImplDX11_NewFrame();
    fn ImGui_ImplDX11_RenderDrawData(draw_data: *const imgui::DrawData);
}
