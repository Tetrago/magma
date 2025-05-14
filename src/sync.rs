use crate::Device;
use crate::Result;
use std::ptr::null;
use std::ptr::null_mut;
use std::sync::Arc;
use vulkan_sys::call;
use vulkan_sys::vk;

#[derive(magma_proc::Object)]
pub struct Fence {
    #[object]
    handle: vk::Fence,
    device: Arc<Device>,
}

impl Fence {
    pub fn new(device: Arc<Device>) -> Result<Self> {
        Self::new_with_flags(device, 0)
    }

    pub fn new_signaled(device: Arc<Device>) -> Result<Self> {
        Self::new_with_flags(device, vk::FENCE_CREATE_SIGNALED_BIT)
    }

    fn new_with_flags(device: Arc<Device>, flags: vk::EnumFlags) -> Result<Self> {
        let create_info = vk::FenceCreateInfo::default().flags(flags);

        let mut handle: vk::Fence = null_mut();
        call!(vk::create_fence(
            device.handle(),
            &create_info,
            null(),
            &mut handle
        ))?;

        Ok(Self { handle, device })
    }

    pub fn wait(&self) -> Result<()> {
        self.wait_for(u64::MAX)
    }

    pub fn wait_for(&self, timeout: u64) -> Result<()> {
        call!(vk::wait_for_fences(
            self.device.handle(),
            1,
            &self.handle,
            vk::TRUE,
            timeout
        ))?;

        Ok(())
    }

    pub fn reset(&mut self) -> Result<()> {
        call!(vk::reset_fences(self.device.handle(), 1, &self.handle))?;
        Ok(())
    }
}

impl Drop for Fence {
    fn drop(&mut self) {
        unsafe {
            vk::destroy_fence(self.device.handle(), self.handle, null());
        }
    }
}

#[derive(magma_proc::Object)]
pub struct Semaphore {
    #[object]
    handle: vk::Semaphore,
    device: Arc<Device>,
}

impl Semaphore {
    pub fn new(device: Arc<Device>) -> Result<Self> {
        let create_info = vk::SemaphoreCreateInfo::default();

        let mut handle: vk::Semaphore = null_mut();
        call!(vk::create_semaphore(
            device.handle(),
            &create_info,
            null(),
            &mut handle
        ))?;

        Ok(Self { handle, device })
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe {
            vk::destroy_semaphore(self.device.handle(), self.handle, null());
        }
    }
}
