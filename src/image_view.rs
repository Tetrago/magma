use crate::Device;
use crate::Result;
use std::ptr::null;
use std::ptr::null_mut;
use std::sync::Arc;
use vulkan_sys::call;
use vulkan_sys::vk;

#[derive(magma_proc::Object)]
pub struct ImageView {
    #[object]
    handle: vk::ImageView,
    device: Arc<Device>,
}

impl ImageView {
    pub fn new(device: Arc<Device>, create_info: vk::ImageViewCreateInfo) -> Result<Self> {
        let mut handle: vk::ImageView = null_mut();
        call!(vk::create_image_view(
            device.handle(),
            &create_info,
            null(),
            &mut handle
        ))?;

        Ok(Self { handle, device })
    }
}

impl Drop for ImageView {
    fn drop(&mut self) {
        unsafe {
            vk::destroy_image_view(self.device.handle(), self.handle, null());
        }
    }
}
