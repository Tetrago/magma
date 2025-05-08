use crate::Instance;
use crate::Result;
use crate::Swapchain;
use crate::predicate;
use std::fmt;
use std::ptr::null;
use std::ptr::null_mut;
use vulkan_sys::call;
use vulkan_sys::to_string;
use vulkan_sys::vk;

#[derive(Debug, Clone)]
pub struct PhysicalDevice {
    pub handle: vk::PhysicalDevice,
    pub properties: vk::PhysicalDeviceProperties,
    pub features: vk::PhysicalDeviceFeatures,
    pub queue_families: Vec<vk::QueueFamilyProperties>,
    pub extensions: Vec<vk::ExtensionProperties>,
}

impl PhysicalDevice {
    pub fn selector(instance: &Instance) -> Result<Selector> {
        Selector::new(instance)
            .map(|selector| selector.bias(|device| device.properties.limits.max_image_dimension2_d))
    }
}

impl PartialEq for PhysicalDevice {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl TryFrom<vk::PhysicalDevice> for PhysicalDevice {
    type Error = crate::Error;

    fn try_from(value: vk::PhysicalDevice) -> Result<Self> {
        let mut properties = Default::default();
        let mut features = Default::default();

        unsafe {
            vk::get_physical_device_properties(value, &mut properties);
            vk::get_physical_device_features(value, &mut features);
        }

        let mut count = 0u32;
        unsafe {
            vk::get_physical_device_queue_family_properties(value, &mut count, null_mut());
        }

        let mut queue_families = vec![vk::QueueFamilyProperties::default(); count as usize];
        unsafe {
            vk::get_physical_device_queue_family_properties(
                value,
                &mut count,
                queue_families.as_mut_ptr(),
            );
        }

        let mut count = 0u32;
        call!(vk::enumerate_device_extension_properties(
            value,
            null(),
            &mut count,
            null_mut()
        ))?;

        let mut extensions = vec![vk::ExtensionProperties::default(); count as usize];
        call!(vk::enumerate_device_extension_properties(
            value,
            null(),
            &mut count,
            extensions.as_mut_ptr(),
        ))?;

        Ok(Self {
            handle: value,
            properties,
            features,
            queue_families,
            extensions,
        })
    }
}

impl fmt::Display for PhysicalDevice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.properties)
    }
}

#[derive(Clone)]
pub struct Selector {
    devices: Vec<(u32, PhysicalDevice)>,
}

impl Selector {
    fn new(instance: &Instance) -> Result<Self> {
        instance
            .enumerate_physical_devices()
            .map(|devices| devices.into_iter().map(|x| (0, x)).collect())
            .map(|devices| Self { devices })
    }

    pub fn exclude(mut self, device: &PhysicalDevice) -> Self {
        self.devices = self
            .devices
            .into_iter()
            .filter(|(_, x)| x != device)
            .collect();

        self
    }

    pub fn filter<P>(mut self, mut predicate: P) -> Self
    where
        P: FnMut(&PhysicalDevice) -> bool,
    {
        self.devices = self
            .devices
            .into_iter()
            .filter(|(_, device)| predicate(device))
            .collect();

        self
    }

    pub fn bias<F>(mut self, mut func: F) -> Self
    where
        F: FnMut(&PhysicalDevice) -> u32,
    {
        self.devices
            .iter_mut()
            .for_each(|(score, device)| *score += func(device));

        self
    }

    pub fn require_graphics_queue_family(self) -> Self {
        let mut predicate = predicate::queue_family_graphics_support();
        self.filter(|device| device.queue_families.iter().any(&mut predicate))
    }

    pub fn require_present_support(self, surface: vk::SurfaceKHR) -> Self {
        self.filter(|device| {
            let mut predicate = predicate::queue_family_index_present_support(device, surface);

            device
                .queue_families
                .iter()
                .enumerate()
                .any(|(i, _)| predicate(i as u32))
        })
    }

    pub fn require_swapchain_support(self, surface: vk::SurfaceKHR) -> Self {
        self.filter(|device| {
            device
                .extensions
                .iter()
                .any(|props| props.to_string() == to_string(vk::KHR_SWAPCHAIN_EXTENSION_NAME))
        })
        .filter(|device| {
            if let Ok(details) = Swapchain::query_details(device, surface) {
                !details.surface_formats.is_empty() && !details.present_modes.is_empty()
            } else {
                false
            }
        })
    }

    pub fn require_extensions(self, extensions: &[String]) -> Self {
        self.filter(|device| {
            extensions.iter().all(|name| {
                device
                    .extensions
                    .iter()
                    .any(|props| props.to_string() == *name)
            })
        })
    }

    pub fn prefer_discrete(self) -> Self {
        self.bias(|device| {
            if device.properties.device_type & vk::PHYSICAL_DEVICE_TYPE_DISCRETE_GPU != 0 {
                1000
            } else {
                0
            }
        })
    }

    pub fn select(mut self) -> Option<PhysicalDevice> {
        self.devices.sort_by(|(a, _), (b, _)| a.cmp(b));
        self.devices.into_iter().next().map(|(_, x)| x)
    }
}
