use crate::CommandBuffer;
use crate::Result;
use crate::sync::Fence;
use std::mem::transmute_copy;
use std::ptr::null_mut;
use vulkan_sys::call;
use vulkan_sys::vk;

pub trait QueueSubmit: Sized {
    fn submit(
        &self,
        submit_info: vk::SubmitInfo,
        command_buffer: &CommandBuffer,
        fence: Option<&Fence>,
    ) -> Result<()>;
}

impl QueueSubmit for vk::Queue {
    fn submit(
        &self,
        submit_info: vk::SubmitInfo,
        command_buffers: &CommandBuffer,
        fence: Option<&Fence>,
    ) -> Result<()> {
        let command_buffer_handle = command_buffers.handle();

        call!(vk::queue_submit(
            transmute_copy(self),
            1,
            &submit_info
                .command_buffer_count(1)
                .command_buffers(&command_buffer_handle),
            fence.map(Fence::handle).unwrap_or(null_mut())
        ))?;

        Ok(())
    }
}

pub trait QueueWait: Sized {
    fn wait(&self) -> Result<()>;
}

impl QueueWait for vk::Queue {
    fn wait(&self) -> Result<()> {
        call!(vk::queue_wait_idle(transmute_copy(self)))?;
        Ok(())
    }
}
