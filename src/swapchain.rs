use crate::Device;
use crate::PhysicalDevice;
use crate::Result;
use crate::sync::Fence;
use crate::sync::Semaphore;
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
    surface: vk::SurfaceKHR,
    surface_format: vk::SurfaceFormatKHR,
    present_mode: vk::PresentModeKHR,
    extent: vk::Extent2D,
    shared_queue_families: Vec<u32>,
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

        let mut obj: Self = Self {
            handle: null_mut(),
            device,
            images: Vec::new(),
            surface,
            surface_format: builder
                .preferred_surface_format
                .unwrap_or(vk::SurfaceFormatKHR {
                    format: vk::FORMAT_UNDEFINED,
                    color_space: vk::COLOR_SPACE_SRGB_NONLINEAR_KHR,
                }),
            present_mode: builder
                .preferred_present_mode
                .unwrap_or(vk::PRESENT_MODE_FIFO_KHR),
            extent: vk::Extent2D::default(),
            shared_queue_families: builder
                .shared_queue_families
                .unwrap_or(HashSet::new())
                .into_iter()
                .collect(),
        };

        obj.recreate(width, height)?;
        Ok(obj)
    }

    pub fn recreate(&mut self, width: u32, height: u32) -> Result<()> {
        let details = Swapchain::query_details(self.device.physical_device(), self.surface)?;

        self.surface_format = details
            .surface_formats
            .iter()
            .find(|x| {
                self.surface_format.format == x.format
                    && self.surface_format.color_space == x.color_space
            })
            .cloned()
            .unwrap_or(details.surface_formats[0]);

        if !details.present_modes.contains(&self.present_mode) {
            self.present_mode = vk::PRESENT_MODE_FIFO_KHR;
        }

        self.extent = vk::Extent2D {
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

        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(self.surface)
            .min_image_count(image_count)
            .image_format(self.surface_format.format)
            .image_color_space(self.surface_format.color_space)
            .image_extent(self.extent)
            .image_array_layers(1)
            .image_usage(vk::IMAGE_USAGE_COLOR_ATTACHMENT_BIT)
            .pre_transform(details.capabilities.current_transform)
            .composite_alpha(vk::COMPOSITE_ALPHA_OPAQUE_BIT_KHR)
            .present_mode(self.present_mode)
            .clipped(vk::TRUE)
            .old_swapchain(self.handle)
            .image_sharing_mode(if self.shared_queue_families.is_empty() {
                vk::SHARING_MODE_EXCLUSIVE
            } else {
                vk::SHARING_MODE_CONCURRENT
            })
            .queue_family_index_count(self.shared_queue_families.len() as u32)
            .queue_family_indices(self.shared_queue_families.as_ptr());

        let mut handle: vk::SwapchainKHR = null_mut();
        call!(vk::create_swapchain_khr(
            self.device.handle(),
            &create_info,
            null(),
            &mut handle
        ))?;

        let mut count = 0u32;
        call!(vk::get_swapchain_images_khr(
            self.device.handle(),
            handle,
            &mut count,
            null_mut()
        ))?;

        let mut images = vec![null_mut(); count as usize];
        call!(vk::get_swapchain_images_khr(
            self.device.handle(),
            handle,
            &mut count,
            images.as_mut_ptr()
        ))?;

        if !self.handle.is_null() {
            unsafe {
                let _ = vk::device_wait_idle(self.device.handle());
                self.images.clear();
                vk::destroy_swapchain_khr(self.device.handle(), self.handle, null());
            }
        }

        self.handle = handle;
        self.images = images;

        Ok(())
    }

    pub fn acquire(&self, semaphore: Option<&Semaphore>, fence: Option<&Fence>) -> Result<u32> {
        let mut index = 0u32;
        call!(vk::acquire_next_image_khr(
            self.device.handle(),
            self.handle,
            u64::MAX,
            semaphore.map(Semaphore::handle).unwrap_or(null_mut()),
            fence.map(Fence::handle).unwrap_or(null_mut()),
            &mut index,
        ))?;

        Ok(index)
    }

    pub fn images(&self) -> &[vk::Image] {
        &self.images
    }

    pub fn format(&self) -> vk::Format {
        self.surface_format.format
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
