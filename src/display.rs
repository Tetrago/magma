use crate::Framebuffer;
use crate::ImageView;
use crate::RenderPass;
use crate::Result;
use crate::Swapchain;
use std::sync::Arc;
use vulkan_sys::vk;

#[derive(magma_proc::Object)]
#[object(builder = Builder)]
pub struct Display {
    swapchain: Swapchain,
    render_pass: Arc<RenderPass>,
    image_views: Vec<ImageView>,
    framebuffers: Vec<Framebuffer>,
}

impl Display {
    fn rebuild_components(&mut self) -> Result<()> {
        self.framebuffers.clear();
        self.image_views.clear();

        let create_info = vk::ImageViewCreateInfo::default()
            .view_type(vk::IMAGE_VIEW_TYPE_2D)
            .format(self.swapchain.format())
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

        self.image_views = self
            .swapchain
            .images()
            .iter()
            .map(|image| ImageView::new(self.swapchain.device.clone(), create_info.image(*image)))
            .collect::<Result<Vec<ImageView>>>()?;

        let extent = self.swapchain.extent();

        self.framebuffers = self
            .image_views
            .iter()
            .map(|image_view| {
                Framebuffer::builder()
                    .device(self.swapchain.device.clone())
                    .render_pass(self.render_pass.clone())
                    .push_attachments(image_view.handle())
                    .width(extent.width)
                    .height(extent.height)
                    .build()
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(())
    }

    fn new(builder: Builder) -> Result<Self> {
        let swapchain = builder.swapchain.unwrap();
        let render_pass = builder.render_pass.unwrap();

        let mut obj = Self {
            swapchain,
            render_pass,
            image_views: Vec::new(),
            framebuffers: Vec::new(),
        };

        obj.rebuild_components()?;
        Ok(obj)
    }

    pub fn recreate(&mut self, width: u32, height: u32) -> Result<()> {
        self.swapchain.recreate(width, height)?;
        self.rebuild_components()?;
        Ok(())
    }

    pub fn swapchain(&self) -> &Swapchain {
        &self.swapchain
    }

    pub fn framebuffers(&self) -> &[Framebuffer] {
        &self.framebuffers
    }
}

#[derive(magma_proc::Builder)]
#[builder(target = Display)]
pub struct Builder {
    #[builder(required)]
    swapchain: Option<Swapchain>,
    #[builder(required)]
    render_pass: Option<Arc<RenderPass>>,
}
