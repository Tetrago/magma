use magma::CommandBuffer;
use magma::CommandPool;
use magma::Device;
use magma::Framebuffer;
use magma::ImageView;
use magma::Instance;
use magma::PhysicalDevice;
use magma::Pipeline;
use magma::RenderPass;
use magma::Result;
use magma::Swapchain;
use magma::call;
use magma::predicate;
use magma::sync::Fence;
use magma::sync::Semaphore;
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

    {
        let device_extensions = [to_string(vk::KHR_SWAPCHAIN_EXTENSION_NAME)];

        let physical_device = PhysicalDevice::selector(&instance)?
            .require_graphics_queue_family()
            .require_present_support(surface)
            .prefer_discrete()
            .require_swapchain_support(surface)
            .select()
            .expect("Could not find suitable device");

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

        let mut queue: Option<vk::Queue> = None;
        let device = Arc::new(
            Device::builder()
                .physical_device(physical_device)
                .order_queue(queue_family, &mut queue)
                .extend_extensions(device_extensions)
                .build()?,
        );

        let queue = queue.unwrap();

        let (width, height) = window.get_framebuffer_size();

        let mut swapchain = Swapchain::builder()
            .device(device.clone())
            .surface(surface)
            .preferred_surface_format(vk::SurfaceFormatKHR {
                format: vk::FORMAT_B8G8R8A8_SRGB,
                color_space: vk::COLOR_SPACE_SRGB_NONLINEAR_KHR,
            })
            .preferred_present_mode(vk::PRESENT_MODE_MAILBOX_KHR)
            .preferred_extent(width as u32, height as u32)
            .build()?;

        let mut color_attachment = 0u32;

        let render_pass = Arc::new(
            RenderPass::builder()
                .device(device.clone())
                .attach(
                    vk::AttachmentDescription::default()
                        .format(swapchain.format())
                        .samples(vk::SAMPLE_COUNT_1_BIT)
                        .load_op(vk::ATTACHMENT_LOAD_OP_CLEAR)
                        .store_op(vk::ATTACHMENT_STORE_OP_STORE)
                        .stencil_load_op(vk::ATTACHMENT_LOAD_OP_DONT_CARE)
                        .stencil_store_op(vk::ATTACHMENT_STORE_OP_DONT_CARE)
                        .initial_layout(vk::IMAGE_LAYOUT_UNDEFINED)
                        .final_layout(vk::IMAGE_LAYOUT_PRESENT_SRC_KHR),
                    &mut color_attachment,
                )
                .subpass(|b| {
                    b.color_attachment(color_attachment, vk::IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL)
                })
                .push_dependencies(
                    vk::SubpassDependency::default()
                        .src_subpass(vk::SUBPASS_EXTERNAL as u32)
                        .dst_subpass(0)
                        .src_stage_mask(vk::PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT)
                        .src_access_mask(0)
                        .dst_stage_mask(vk::PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT)
                        .dst_access_mask(vk::ACCESS_COLOR_ATTACHMENT_WRITE_BIT),
                )
                .build()?,
        );

        let pipeline = Pipeline::builder()
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

        let command_pool = CommandPool::builder()
            .device(device.clone())
            .queue_family_index(queue_family)
            .reset()
            .build()?;

        const IMAGES_IN_FLIGHT: usize = 2;

        let mut command_buffers = Vec::<CommandBuffer>::with_capacity(IMAGES_IN_FLIGHT);

        for _ in 0..IMAGES_IN_FLIGHT {
            command_buffers.push(command_pool.get_buffer()?);
        }

        let image_available_semaphores = (0..IMAGES_IN_FLIGHT)
            .map(|_| Semaphore::new(device.clone()))
            .collect::<Result<Vec<_>>>()?;

        let render_finished_semaphores = (0..IMAGES_IN_FLIGHT)
            .map(|_| Semaphore::new(device.clone()))
            .collect::<Result<Vec<_>>>()?;

        let mut in_flight_fences = (0..IMAGES_IN_FLIGHT)
            .map(|_| Fence::new_signaled(device.clone()))
            .collect::<Result<Vec<_>>>()?;

        let mut image_views = Vec::<ImageView>::new();
        let mut framebuffers = Vec::<Framebuffer>::new();

        fn rebuild_framebuffers(
            device: &Arc<Device>,
            render_pass: &Arc<RenderPass>,
            swapchain: &mut Swapchain,
            image_views: &mut Vec<ImageView>,
            framebuffers: &mut Vec<Framebuffer>,
            (width, height): (i32, i32),
        ) -> Result<()> {
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

            *image_views = swapchain
                .images()
                .iter()
                .map(|image| ImageView::new(device.clone(), create_info.image(*image)))
                .collect::<Result<Vec<ImageView>>>()?;

            *framebuffers = image_views
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

            Ok(())
        }

        rebuild_framebuffers(
            &device,
            &render_pass,
            &mut swapchain,
            &mut image_views,
            &mut framebuffers,
            window.get_framebuffer_size(),
        )?;

        let mut current_frame = 0usize;

        while !window.should_close() {
            glfw.poll_events();
            for (_, event) in glfw::flush_messages(&events) {
                match event {
                    _ => {}
                }
            }

            in_flight_fences[current_frame].wait().unwrap();

            let image = match swapchain
                .acquire(Some(&image_available_semaphores[current_frame]), None)
            {
                Ok(index) => index,
                Err(magma::Error::Vulkan(error))
                    if error.0 == vk::ERROR_OUT_OF_DATE_KHR || error.0 == vk::SUBOPTIMAL_KHR =>
                {
                    let (width, height) = window.get_framebuffer_size();

                    swapchain.recreate(width as u32, height as u32)?;
                    rebuild_framebuffers(
                        &device,
                        &render_pass,
                        &mut swapchain,
                        &mut image_views,
                        &mut framebuffers,
                        (width, height),
                    )?;

                    continue;
                }
                _ => panic!("Unknown swapchain error"),
            };

            in_flight_fences[current_frame].reset().unwrap();

            let cmd = &mut command_buffers[current_frame];
            cmd.reset()?;
            cmd.begin_once()?;

            unsafe {
                let clear_value = vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0],
                    },
                };

                let render_pass = vk::RenderPassBeginInfo::default()
                    .render_pass(render_pass.handle())
                    .framebuffer(framebuffers[image as usize].handle())
                    .render_area(vk::Rect2D::default().extent(swapchain.extent()))
                    .clear_value_count(1)
                    .clear_values(&clear_value);

                cmd.begin_render_pass(&render_pass, vk::SUBPASS_CONTENTS_INLINE);
                cmd.bind_pipeline(vk::PIPELINE_BIND_POINT_GRAPHICS, pipeline.handle());

                let viewport = vk::Viewport::default()
                    .x(0.0)
                    .y(0.0)
                    .width(swapchain.extent().width as f32)
                    .height(swapchain.extent().height as f32)
                    .min_depth(0.0)
                    .max_depth(1.0);
                cmd.set_viewport(0, 1, &viewport);

                let scissor = vk::Rect2D::default().extent(swapchain.extent());
                cmd.set_scissor(0, 1, &scissor);

                cmd.draw(3, 1, 0, 0);
                cmd.end_render_pass();
            }

            cmd.end()?;

            let image_available_semaphore = image_available_semaphores[current_frame].handle();
            let render_finished_semaphore = render_finished_semaphores[current_frame].handle();
            let cmd_handle = cmd.handle();

            let submit_info = vk::SubmitInfo::default()
                .wait_semaphore_count(1)
                .wait_semaphores(&image_available_semaphore)
                .wait_dst_stage_mask(&vk::PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT)
                .command_buffer_count(1)
                .command_buffers(&cmd_handle)
                .signal_semaphore_count(1)
                .signal_semaphores(&render_finished_semaphore);

            call!(vk::queue_submit(
                queue,
                1,
                &submit_info,
                in_flight_fences[current_frame].handle()
            ))
            .unwrap();

            let swapchain_handle = swapchain.handle();

            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphore_count(1)
                .wait_semaphores(&render_finished_semaphore)
                .swapchain_count(1)
                .swapchains(&swapchain_handle)
                .image_indices(&image);

            call!(vk::queue_present_khr(queue, &present_info)).unwrap();

            current_frame = (current_frame + 1) % IMAGES_IN_FLIGHT;
        }

        call!(vk::device_wait_idle(device.handle())).unwrap()
    }

    unsafe {
        vk::destroy_surface_khr(instance.handle(), surface, std::ptr::null());
    }

    Ok(())
}
