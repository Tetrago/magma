use crate::Device;
use crate::RenderPass;
use crate::Result;
use std::ptr::null;
use std::ptr::null_mut;
use std::sync::Arc;
use vulkan_sys::call;
use vulkan_sys::vk;

#[derive(magma_proc::Object)]
#[object(builder = Builder)]
pub struct Framebuffer {
    #[object]
    handle: vk::Framebuffer,
    device: Arc<Device>,
    _render_pass: Arc<RenderPass>,
}

impl Framebuffer {
    fn new(builder: Builder) -> Result<Self> {
        let device = builder.device.unwrap();
        let render_pass = builder.render_pass.unwrap();

        let create_info = vk::FramebufferCreateInfo::default()
            .render_pass(render_pass.handle())
            .attachment_count(builder.attachments.len() as u32)
            .attachments(builder.attachments.as_ptr())
            .width(builder.width)
            .height(builder.height)
            .layers(builder.layers);

        let mut handle: vk::Framebuffer = null_mut();
        call!(vk::create_framebuffer(
            device.handle(),
            &create_info,
            null(),
            &mut handle
        ))?;

        Ok(Self {
            handle,
            device,
            _render_pass: render_pass,
        })
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            vk::destroy_framebuffer(self.device.handle(), self.handle, null());
        }
    }
}

#[derive(magma_proc::Builder)]
#[builder(target = Framebuffer)]
pub struct Builder {
    #[builder(required)]
    device: Option<Arc<Device>>,
    #[builder(required)]
    render_pass: Option<Arc<RenderPass>>,
    attachments: Vec<vk::ImageView>,
    width: u32,
    height: u32,
    #[builder(default = 1)]
    layers: u32,
}
