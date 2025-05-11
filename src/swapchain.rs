use crate::Device;
use crate::PhysicalDevice;
use crate::Result;
use std::collections::HashSet;
use std::ptr::null;
use std::ptr::null_mut;
use std::sync::Arc;
use vulkan_sys::call;
use vulkan_sys::vk;

#[derive(Debug, Default, Clone)]
pub struct SwapchainDetails {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub surface_formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

#[derive(magma_proc::Object)]
#[object(builder = Builder)]
pub struct Swapchain {
    #[object]
    handle: vk::SwapchainKHR,
    device: Arc<Device>,
    images: Vec<vk::Image>,
    format: vk::Format,
    extent: vk::Extent2D,
}

impl Swapchain {
    pub fn query_details(
        device: &PhysicalDevice,
        surface: vk::SurfaceKHR,
    ) -> Result<SwapchainDetails> {
        let mut details = SwapchainDetails::default();

        call!(vk::get_physical_device_surface_capabilities_khr(
            device.handle,
            surface,
            &mut details.capabilities,
        ))?;

        let mut count = 0u32;
        call!(vk::get_physical_device_surface_formats_khr(
            device.handle,
            surface,
            &mut count,
            null_mut()
        ))?;

        details
            .surface_formats
            .resize(count as usize, Default::default());
        call!(vk::get_physical_device_surface_formats_khr(
            device.handle,
            surface,
            &mut count,
            details.surface_formats.as_mut_ptr()
        ))?;

        let mut count = 0u32;
        call!(vk::get_physical_device_surface_present_modes_khr(
            device.handle,
            surface,
            &mut count,
            null_mut()
        ))?;

        details
            .present_modes
            .resize(count as usize, Default::default());
        call!(vk::get_physical_device_surface_present_modes_khr(
            device.handle,
            surface,
            &mut count,
            details.present_modes.as_mut_ptr()
        ))?;

        Ok(details)
    }

    fn new(builder: Builder) -> Result<Self> {
        let device = builder.device.unwrap();
        let surface = builder.surface.unwrap();
        let (width, height) = builder.preferred_extent.unwrap();

        let details = Swapchain::query_details(device.physical_device(), surface)?;

        let surface_format = builder
            .preferred_surface_format
            .and_then(|surface_format| {
                details.surface_formats.iter().find(|x| {
                    surface_format.format == x.format && surface_format.color_space == x.color_space
                })
            })
            .unwrap_or(&details.surface_formats[0]);

        let present_mode = builder
            .preferred_present_mode
            .and_then(|present_mode| {
                if details.present_modes.contains(&present_mode) {
                    Some(present_mode)
                } else {
                    None
                }
            })
            .unwrap_or(vk::PRESENT_MODE_FIFO_KHR);

        let extent = vk::Extent2D {
            width: width.clamp(
                details.capabilities.min_image_extent.width,
                details.capabilities.max_image_extent.width,
            ),
            height: height.clamp(
                details.capabilities.min_image_extent.height,
                details.capabilities.max_image_extent.height,
            ),
        };

        let image_count = {
            let image_count = details.capabilities.min_image_count + 1;

            if details.capabilities.max_image_count > 0 {
                image_count.min(details.capabilities.max_image_count)
            } else {
                image_count
            }
        };

        let mut create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::IMAGE_USAGE_COLOR_ATTACHMENT_BIT)
            .image_sharing_mode(vk::SHARING_MODE_EXCLUSIVE)
            .pre_transform(details.capabilities.current_transform)
            .composite_alpha(vk::COMPOSITE_ALPHA_OPAQUE_BIT_KHR)
            .present_mode(present_mode)
            .clipped(vk::TRUE)
            .old_swapchain(null_mut());

        let mut indices: Option<Vec<u32>> = None;
        if let Some(set) = builder.shared_queue_families {
            indices.replace(set.into_iter().collect());

            create_info = create_info
                .queue_family_index_count(indices.as_ref().unwrap().len() as u32)
                .queue_family_indices(indices.unwrap().as_ptr())
                .image_sharing_mode(vk::SHARING_MODE_CONCURRENT);
        }

        let mut handle: vk::SwapchainKHR = null_mut();
        call!(vk::create_swapchain_khr(
            device.handle(),
            &create_info,
            null(),
            &mut handle
        ))?;

        let mut count = 0u32;
        call!(vk::get_swapchain_images_khr(
            device.handle(),
            handle,
            &mut count,
            null_mut()
        ))?;

        let mut images: Vec<vk::Image> = vec![null_mut(); count as usize];
        call!(vk::get_swapchain_images_khr(
            device.handle(),
            handle,
            &mut count,
            images.as_mut_ptr()
        ))?;

        Ok(Self {
            handle,
            device,
            images,
            format: surface_format.format,
            extent,
        })
    }

    pub fn images(&self) -> &[vk::Image] {
        &self.images
    }

    pub fn format(&self) -> vk::Format {
        self.format
    }

    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            vk::destroy_swapchain_khr(self.device.handle(), self.handle, null());
        }
    }
}

#[derive(Clone, magma_proc::Builder)]
#[builder(target = Swapchain)]
pub struct Builder {
    #[builder(required)]
    device: Option<Arc<Device>>,
    #[builder(required)]
    surface: Option<vk::SurfaceKHR>,
    preferred_surface_format: Option<vk::SurfaceFormatKHR>,
    preferred_present_mode: Option<vk::PresentModeKHR>,
    #[builder(required, skip)]
    preferred_extent: Option<(u32, u32)>,
    shared_queue_families: Option<HashSet<u32>>,
}

impl Builder {
    pub fn preferred_extent(mut self, width: u32, height: u32) -> Self {
        self.preferred_extent.replace((width, height));
        self
    }
}
