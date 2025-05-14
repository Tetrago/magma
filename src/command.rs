use crate::Device;
use crate::Result;
use std::collections::LinkedList;
use std::ops::Deref;
use std::ops::DerefMut;
use std::pin::Pin;
use std::ptr::null;
use std::ptr::null_mut;
use std::sync::Arc;
use std::sync::Mutex;
use vulkan_sys::call;
use vulkan_sys::internal;
use vulkan_sys::vk;

#[derive(magma_proc::Object)]
#[object(builder = Builder)]
pub struct CommandPool {
    #[object]
    handle: vk::CommandPool,
    device: Arc<Device>,
    buffers: Pin<Box<Mutex<LinkedList<vk::CommandBuffer>>>>,
}

impl CommandPool {
    fn new(builder: Builder) -> Result<Self> {
        let device = builder.device.unwrap();

        let create_info = vk::CommandPoolCreateInfo::default()
            .flags(builder.flags)
            .queue_family_index(builder.queue_family_index);

        let mut handle: vk::CommandPool = null_mut();
        call!(vk::create_command_pool(
            device.handle(),
            &create_info,
            null(),
            &mut handle
        ))?;

        Ok(Self {
            handle,
            device,
            buffers: Box::pin(Mutex::default()),
        })
    }

    pub fn get_buffer(&mut self) -> Result<CommandBuffer<'_>> {
        if let Some(handle) = self.buffers.lock().unwrap().pop_front() {
            return Ok(CommandBuffer::new(&self.buffers, handle));
        }

        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.handle)
            .level(vk::COMMAND_BUFFER_LEVEL_PRIMARY)
            .command_buffer_count(1);

        let mut handle: vk::CommandBuffer = null_mut();
        call!(vk::allocate_command_buffers(
            self.device.handle(),
            &alloc_info,
            &mut handle
        ))?;

        Ok(CommandBuffer::new(&self.buffers, handle))
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            vk::destroy_command_pool(self.device.handle(), self.handle, null());
        }
    }
}

#[derive(magma_proc::Builder, Clone)]
#[builder(target = CommandPool)]
pub struct Builder {
    #[builder(required)]
    device: Option<Arc<Device>>,
    flags: vk::EnumFlags,
    queue_family_index: u32,
}

impl Builder {
    pub fn reset(mut self) -> Self {
        self.flags |= vk::COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT;
        self
    }
}

pub struct CommandBuffer<'a> {
    buffers: &'a Mutex<LinkedList<vk::CommandBuffer>>,
    internal: internal::CommandBuffer,
}

impl<'a> CommandBuffer<'a> {
    fn new(buffers: &'a Mutex<LinkedList<vk::CommandBuffer>>, handle: vk::CommandBuffer) -> Self {
        Self {
            buffers,
            internal: handle.into(),
        }
    }

    pub fn handle(&self) -> vk::CommandBuffer {
        self.internal.handle()
    }

    pub fn begin(&mut self) -> Result<()> {
        self.begin_with_flags(0)
    }

    pub fn begin_once(&mut self) -> Result<()> {
        self.begin_with_flags(vk::COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT)
    }

    fn begin_with_flags(&mut self, flags: vk::EnumFlags) -> Result<()> {
        let begin_info = vk::CommandBufferBeginInfo::default().flags(flags);
        call!(vk::begin_command_buffer(self.handle(), &begin_info))?;

        Ok(())
    }

    pub fn end(&mut self) -> Result<()> {
        call!(vk::end_command_buffer(self.handle()))?;
        Ok(())
    }

    pub fn reset(&mut self) -> Result<()> {
        call!(vk::reset_command_buffer(self.handle(), 0))?;
        Ok(())
    }
}

impl Drop for CommandBuffer<'_> {
    fn drop(&mut self) {
        self.buffers.lock().unwrap().push_back(self.handle());
    }
}

impl Deref for CommandBuffer<'_> {
    type Target = internal::CommandBuffer;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl DerefMut for CommandBuffer<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.internal
    }
}
