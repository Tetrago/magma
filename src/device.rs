use crate::Error;
use crate::PhysicalDevice;
use crate::Result;
use std::collections::HashMap;
use std::ffi::CString;
use std::ptr::null;
use std::ptr::null_mut;
use vulkan_sys::call;
use vulkan_sys::vk;

#[derive(magma_proc::Object)]
pub struct Device {
    #[object]
    handle: vk::Device,
    physical_device: PhysicalDevice,
}

impl Device {
    pub fn builder<'a>() -> Builder<'a> {
        Builder::default()
    }

    fn new(builder: Builder) -> Result<Self> {
        let priority: f32 = 1.0;
        let physical_device = builder.physical_device.unwrap();

        let queue_create_infos = builder
            .queues
            .iter()
            .map(|(&index, handles)| {
                let count = handles.len() as u32;
                let queue = &physical_device.queue_families[index as usize];

                if count > queue.queue_count {
                    Err(Error::InvalidQueueCount {
                        requested: count,
                        index,
                        available: queue.queue_count,
                    })
                } else {
                    Ok(vk::DeviceQueueCreateInfo::default()
                        .queue_family_index(index)
                        .queue_count(count)
                        .queue_priorities(&priority))
                }
            })
            .collect::<Result<Vec<_>>>()?;

        let features = vk::PhysicalDeviceFeatures::default();

        let extensions: Vec<_> = builder
            .extensions
            .into_iter()
            .map(|name| CString::new(name).expect("Invalid device extension name"))
            .collect();
        let extensions: Vec<_> = extensions.iter().map(|x| x.as_ptr()).collect();

        let create_info = vk::DeviceCreateInfo::default()
            .queue_create_info_count(queue_create_infos.len() as u32)
            .queue_create_infos(queue_create_infos.as_ptr())
            .enabled_features(&features)
            .enabled_extension_count(extensions.len() as u32)
            .enabled_extension_names(extensions.as_ptr());

        let mut handle: vk::Device = null_mut();
        call!(vk::create_device(
            physical_device.handle,
            &create_info,
            null(),
            &mut handle
        ))?;

        builder.queues.into_iter().for_each(|(index, handles)| {
            handles.into_iter().enumerate().for_each(|(i, out)| {
                let mut queue: vk::Queue = null_mut();
                unsafe {
                    vk::get_device_queue(handle, index, i as u32, &mut queue);
                }

                out.replace(queue);
            });
        });

        Ok(Self {
            handle,
            physical_device,
        })
    }

    pub fn wait(&self) -> Result<()> {
        call!(vk::device_wait_idle(self.handle))?;
        Ok(())
    }

    pub fn physical_device(&self) -> &PhysicalDevice {
        &self.physical_device
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            let _ = vk::device_wait_idle(self.handle);
            vk::destroy_device(self.handle, null());
        }
    }
}

#[derive(magma_proc::Builder)]
#[builder(target = Device)]
pub struct Builder<'a> {
    #[builder(required)]
    physical_device: Option<PhysicalDevice>,
    queues: HashMap<u32, Vec<&'a mut Option<vk::Queue>>>,
    extensions: Vec<String>,
}

impl<'a> Builder<'a> {
    pub fn order_queue(mut self, queue_family_index: u32, out: &'a mut Option<vk::Queue>) -> Self {
        self.queues
            .entry(queue_family_index)
            .or_insert(Vec::new())
            .push(out);

        self
    }
}
