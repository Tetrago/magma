use magma::Instance;
use magma::PhysicalDevice;
use magma::Result;
use magma::vk;

fn main() -> Result<()> {
    let mut glfw = glfw::init(glfw::fail_on_errors).unwrap();
    glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));

    let (window, events) = glfw
        .create_window(1280, 720, "Magma", glfw::WindowMode::Windowed)
        .unwrap();

    let instance = Instance::builder()
        .app_name("triangle".to_owned())
        .app_version(0, 1, 0)
        .engine_name("magma".to_owned())
        .engine_version(0, 1, 0)
        .extend_extensions(glfw.get_required_instance_extensions().unwrap().into_iter())
        .build()
        .expect("Failed to initialize Vulkan instance");

    let mut surface: vk::SurfaceKHR = std::ptr::null_mut();
    unsafe {
        window
            .create_window_surface(
                std::mem::transmute_copy(&instance.handle()),
                std::ptr::null(),
                std::mem::transmute_copy(&&mut surface),
            )
            .result()
            .ok();
    }

    let _physical_device = PhysicalDevice::selector(&instance)?
        .require_graphics_queue_family()
        .prefer_discrete()
        .select();

    'main: loop {
        glfw.poll_events();

        for (_, event) in glfw::flush_messages(&events) {
            match event {
                glfw::WindowEvent::Close => break 'main,
                _ => {}
            }
        }
    }

    unsafe {
        vk::destroy_surface_khr(instance.handle(), surface, std::ptr::null());
    }

    Ok(())
}
