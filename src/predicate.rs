use crate::PhysicalDevice;
use vulkan_sys::call;
use vulkan_sys::vk;

pub fn queue_family_graphics_support() -> impl FnMut(&vk::QueueFamilyProperties) -> bool {
    |queue_family| queue_family.queue_flags & vk::QUEUE_GRAPHICS_BIT != 0
}

pub fn queue_family_index_present_support(
    physical_device: &PhysicalDevice,
    surface: vk::SurfaceKHR,
) -> impl FnMut(u32) -> bool {
    let handle = physical_device.handle;

    move |index| {
        let mut present_support: vk::Bool32 = vk::FALSE;

        call!(vk::get_physical_device_surface_support_khr(
            handle,
            index,
            surface,
            &mut present_support,
        ))
        .ok();

        present_support != vk::FALSE
    }
}
