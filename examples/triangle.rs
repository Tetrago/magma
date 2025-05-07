use magma::Device;
use magma::Instance;
use magma::PhysicalDevice;
use magma::Result;
use magma::predicate;
use magma::to_string;
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

    let physical_device = PhysicalDevice::selector(&instance)?
        .require_graphics_queue_family()
        .require_present_support(surface)
        .require_extensions(&[to_string(vk::KHR_SWAPCHAIN_EXTENSION_NAME)])
        .prefer_discrete()
        .select()
        .expect("Could not find suitable device");

    let mut queue: Option<vk::Queue> = None;

    let _device = {
        let queue_family = {
            let mut graphics = predicate::queue_family_graphics_support();
            let mut present =
                predicate::queue_family_index_present_support(&physical_device, surface);

            physical_device
                .queue_families
                .iter()
                .enumerate()
                .find(|&(i, queue_family)| graphics(queue_family) && present(i as u32))
                .expect("Could not find suitable queue")
                .0 as u32
        };

        Device::builder()
            .physical_device(physical_device)
            .order_queue(queue_family, &mut queue)
            .push_extensions(to_string(vk::KHR_SWAPCHAIN_EXTENSION_NAME))
            .build()?;
    };

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
