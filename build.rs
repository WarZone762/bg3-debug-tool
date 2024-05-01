fn main() {
    cc::Build::new()
        .cpp(true)
        .define("IMGUI_IMPL_API", "extern \"C\"")
        .define("ImTextureID", "ImU64")
        .include("third-party/imgui")
        .include("third-party/Vulkan-Headers/include")
        .file("third-party/imgui/backends/imgui_impl_vulkan.cpp")
        .file("third-party/imgui/backends/imgui_impl_dx11.cpp")
        .file("third-party/imgui/backends/imgui_impl_win32.cpp")
        .compile("imgui_backends");
    println!("cargo:rustc-link-lib=vulkan-1");
    embed_resource::compile("assets/res.rc", embed_resource::NONE);
}
