use crate::Instance;
use crate::Result;
use std::fmt;
use std::ptr::null_mut;
use vulkan_sys::call;
use vulkan_sys::vk;

#[derive(Debug, Clone)]
pub struct PhysicalDevice {
    pub handle: vk::PhysicalDevice,
    pub properties: vk::PhysicalDeviceProperties,
    pub features: vk::PhysicalDeviceFeatures,
    pub queue_families: Vec<vk::QueueFamilyProperties>,
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
        let mut queue_families = Vec::<vk::QueueFamilyProperties>::new();

        unsafe {
            vk::get_physical_device_queue_family_properties(value, &mut count, null_mut());

            queue_families.resize(count as usize, Default::default());
            vk::get_physical_device_queue_family_properties(
                value,
                &mut count,
                queue_families.as_mut_ptr(),
            );
        }

        Ok(Self {
            handle: value,
            properties,
            features,
            queue_families,
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
        self.filter(|device| {
            device
                .queue_families
                .iter()
                .any(|queue_family| queue_family.queue_flags & vk::QUEUE_GRAPHICS_BIT == 0)
        })
    }

    pub fn require_present_support(self, surface: vk::SurfaceKHR) -> Self {
        self.filter(|device| {
            device.queue_families.iter().enumerate().any(|(i, _)| {
                let mut present_support: vk::Bool32 = vk::FALSE;

                call!(vk::get_physical_device_surface_support_khr(
                    device.handle,
                    i as u32,
                    surface,
                    &mut present_support,
                ))
                .ok();

                present_support == vk::FALSE
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
