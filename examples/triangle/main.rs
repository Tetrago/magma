use magma::Device;
use magma::Framebuffer;
use magma::ImageView;
use magma::Instance;
use magma::PhysicalDevice;
use magma::Pipeline;
use magma::RenderPass;
use magma::Result;
use magma::Swapchain;
use magma::predicate;
use magma::to_string;
use magma::vk;
use std::sync::Arc;

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

    let device_extensions = [to_string(vk::KHR_SWAPCHAIN_EXTENSION_NAME)];

    let physical_device = PhysicalDevice::selector(&instance)?
        .require_graphics_queue_family()
        .require_present_support(surface)
        .prefer_discrete()
        .require_swapchain_support(surface)
        .select()
        .expect("Could not find suitable device");

    let mut queue: Option<vk::Queue> = None;
    let device = {
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

        Arc::new(
            Device::builder()
                .physical_device(physical_device)
                .order_queue(queue_family, &mut queue)
                .extend_extensions(device_extensions)
                .build()?,
        )
    };

    let (width, height) = window.get_framebuffer_size();

    let swapchain = Swapchain::builder()
        .device(device.clone())
        .surface(surface)
        .preferred_surface_format(vk::SurfaceFormatKHR {
            format: vk::FORMAT_B8G8R8A8_SRGB,
            color_space: vk::COLOR_SPACE_SRGB_NONLINEAR_KHR,
        })
        .preferred_present_mode(vk::PRESENT_MODE_MAILBOX_KHR)
        .preferred_extent(width as u32, height as u32)
        .build()?;

    let image_views = {
        let create_info = vk::ImageViewCreateInfo::default()
            .view_type(vk::IMAGE_VIEW_TYPE_2D)
            .format(swapchain.format())
            .components(vk::ComponentMapping {
                r: vk::COMPONENT_SWIZZLE_IDENTITY,
                g: vk::COMPONENT_SWIZZLE_IDENTITY,
                b: vk::COMPONENT_SWIZZLE_IDENTITY,
                a: vk::COMPONENT_SWIZZLE_IDENTITY,
            })
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::IMAGE_ASPECT_COLOR_BIT)
                    .level_count(1)
                    .layer_count(1),
            );

        swapchain
            .images()
            .iter()
            .map(|image| ImageView::new(device.clone(), create_info.image(*image)))
            .collect::<Result<Vec<ImageView>>>()?
    };

    let mut color_attachment = 0u32;

    let render_pass = Arc::new(
        RenderPass::builder()
            .device(device.clone())
            .attach(
                vk::AttachmentDescription::default()
                    .format(swapchain.format())
                    .samples(vk::SAMPLE_COUNT_1_BIT)
                    .load_op(vk::ATTACHMENT_LOAD_OP_LOAD)
                    .store_op(vk::ATTACHMENT_STORE_OP_STORE)
                    .stencil_load_op(vk::ATTACHMENT_LOAD_OP_DONT_CARE)
                    .stencil_store_op(vk::ATTACHMENT_STORE_OP_DONT_CARE)
                    .initial_layout(vk::IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL)
                    .final_layout(vk::IMAGE_LAYOUT_PRESENT_SRC_KHR),
                &mut color_attachment,
            )
            .subpass(|b| {
                b.color_attachment(color_attachment, vk::IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL)
            })
            .build()?,
    );

    let _pipeline = Pipeline::builder()
        .device(device.clone())
        .render_pass(render_pass.clone())
        .compile_shader(
            vk::SHADER_STAGE_VERTEX_BIT,
            include_bytes!(concat!(
                env!("OUT_DIR"),
                "/shaders/triangle/basic.vertex.spv"
            )),
        )
        .compile_shader(
            vk::SHADER_STAGE_FRAGMENT_BIT,
            include_bytes!(concat!(
                env!("OUT_DIR"),
                "/shaders/triangle/basic.fragment.spv"
            )),
        )
        .dynamic_states(vec![vk::DYNAMIC_STATE_VIEWPORT, vk::DYNAMIC_STATE_SCISSOR])
        .build()?;

    let _framebuffers = image_views
        .iter()
        .map(|image_view| {
            Framebuffer::builder()
                .device(device.clone())
                .render_pass(render_pass.clone())
                .push_attachments(image_view.handle())
                .width(width as u32)
                .height(height as u32)
                .build()
        })
        .collect::<Result<Vec<_>>>()?;

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
