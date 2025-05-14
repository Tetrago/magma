use crate::Device;
use crate::Result;
use std::ptr::null;
use std::ptr::null_mut;
use std::sync::Arc;
use vulkan_sys::call;
use vulkan_sys::vk;

#[derive(magma_proc::Object)]
#[object(builder = Builder)]
pub struct RenderPass {
    #[object]
    handle: vk::RenderPass,
    device: Arc<Device>,
}

impl RenderPass {
    fn new(builder: Builder) -> Result<Self> {
        let device = builder.device.unwrap();

        let subpasses: Vec<_> = builder
            .subpasses
            .iter()
            .map(|subpass| {
                vk::SubpassDescription::default()
                    .pipeline_bind_point(subpass.bind_point)
                    .color_attachment_count(subpass.color_attachments.len() as u32)
                    .color_attachments(subpass.color_attachments.as_ptr())
            })
            .collect();

        let create_info = vk::RenderPassCreateInfo::default()
            .attachment_count(builder.attachments.len() as u32)
            .attachments(builder.attachments.as_ptr())
            .subpass_count(subpasses.len() as u32)
            .subpasses(subpasses.as_ptr())
            .dependency_count(builder.dependencies.len() as u32)
            .dependencies(builder.dependencies.as_ptr());

        let mut handle: vk::RenderPass = null_mut();
        call!(vk::create_render_pass(
            device.handle(),
            &create_info,
            null(),
            &mut handle
        ))?;

        Ok(Self { handle, device })
    }
}

impl Drop for RenderPass {
    fn drop(&mut self) {
        unsafe {
            vk::destroy_render_pass(self.device.handle(), self.handle, null());
        }
    }
}

#[derive(magma_proc::Builder, Clone)]
#[builder(target = RenderPass)]
pub struct Builder {
    #[builder(required)]
    device: Option<Arc<Device>>,
    #[builder(skip)]
    attachments: Vec<vk::AttachmentDescription>,
    #[builder(skip)]
    subpasses: Vec<Subpass>,
    dependencies: Vec<vk::SubpassDependency>,
}

#[derive(magma_proc::Builder, Clone)]
pub struct Subpass {
    #[builder(default = vk::PIPELINE_BIND_POINT_GRAPHICS)]
    bind_point: vk::Enum,
    #[builder(skip)]
    color_attachments: Vec<vk::AttachmentReference>,
}

impl Subpass {
    pub fn color_attachment(mut self, attachment: u32, layout: vk::Enum) -> Self {
        self.color_attachments.push(
            vk::AttachmentReference::default()
                .attachment(attachment)
                .layout(layout),
        );

        self
    }
}

impl Builder {
    pub fn attach(mut self, attachment: vk::AttachmentDescription, index: &mut u32) -> Self {
        *index = self.attachments.len() as u32;
        self.attachments.push(attachment);

        self
    }

    pub fn subpass<F>(mut self, mut f: F) -> Self
    where
        F: FnMut(Subpass) -> Subpass,
    {
        self.subpasses.push(f(Subpass::default()));
        self
    }
}
