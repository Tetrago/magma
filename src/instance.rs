use crate::Error;
use crate::PhysicalDevice;
use crate::Result;
use std::ffi::CString;
use std::ptr::null;
use std::ptr::null_mut;
use vulkan_sys::call;
use vulkan_sys::vk;

#[allow(unused_imports)]
use std::ffi::CStr;
#[allow(unused_imports)]
use std::ffi::c_void;

#[derive(magma_proc::Object)]
#[object(builder = Builder)]
pub struct Instance {
    #[object]
    handle: vk::Instance,
    #[cfg(debug_assertions)]
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

#[cfg(debug_assertions)]
unsafe extern "C" fn debug_callback(
    _severity: vk::DebugUtilsMessageSeverityFlagBitsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagBitsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut c_void,
) -> vk::Bool32 {
    let message = unsafe {
        CStr::from_ptr((*callback_data).message)
            .to_str()
            .unwrap_or("Unknown message")
    };

    eprintln!("[vulkan] {}", message);
    vk::FALSE
}

impl Instance {
    fn validate<T, F>(required: &[String], available: &[T], map_err: F) -> Result<()>
    where
        T: ToString,
        F: FnOnce(Vec<String>) -> Error,
    {
        let missing: Vec<_> = required
            .iter()
            .filter(|&name| !available.iter().any(|props| props.to_string().eq(name)))
            .cloned()
            .collect();

        if !missing.is_empty() {
            Err(map_err(missing))
        } else {
            Ok(())
        }
    }

    fn new(builder: Builder) -> Result<Self> {
        let required_extensions = builder.extensions;
        let required_layers = builder.layers;

        Self::validate(
            &required_extensions,
            &Self::enumerate_extensions()?,
            Error::MissingInstanceExtensions,
        )?;

        Self::validate(
            &required_layers,
            &Self::enumerate_layers()?,
            Error::MissingInstanceExtensions,
        )?;

        let required_extensions: Vec<_> = required_extensions
            .into_iter()
            .map(|name| CString::new(name).expect("Invalid instance extension name"))
            .collect();

        let required_layers: Vec<_> = required_layers
            .into_iter()
            .map(|name| CString::new(name).expect("Invalid instance layer name"))
            .collect();

        #[allow(unused_mut)]
        let mut extensions: Vec<_> = required_extensions
            .iter()
            .map(|name| name.as_ptr())
            .collect();

        #[cfg(debug_assertions)]
        extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION_NAME.as_ptr() as *const _);

        #[allow(unused_mut)]
        let mut layers: Vec<_> = required_layers.iter().map(|name| name.as_ptr()).collect();

        #[cfg(debug_assertions)]
        layers.push(b"VK_LAYER_KHRONOS_validation\0".as_ptr() as *const _);

        let app_info = vk::ApplicationInfo::default()
            .application_name(builder.app_name.as_ptr() as *const _)
            .application_version(builder.api_version)
            .engine_name(builder.engine_name.as_ptr() as *const _)
            .engine_version(builder.engine_version)
            .api_version(builder.api_version);

        #[cfg(debug_assertions)]
        let messenger_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                vk::DEBUG_UTILS_MESSAGE_SEVERITY_WARNING_BIT_EXT
                    | vk::DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT,
            )
            .message_type(
                vk::DEBUG_UTILS_MESSAGE_TYPE_GENERAL_BIT_EXT
                    | vk::DEBUG_UTILS_MESSAGE_TYPE_VALIDATION_BIT_EXT
                    | vk::DEBUG_UTILS_MESSAGE_TYPE_PERFORMANCE_BIT_EXT,
            )
            .pfn_user_callback(Some(debug_callback));

        #[allow(unused_mut)]
        let mut create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_count(extensions.len() as u32)
            .enabled_extension_names(extensions.as_ptr())
            .enabled_layer_count(layers.len() as u32)
            .enabled_layer_names(layers.as_ptr());

        #[cfg(debug_assertions)]
        {
            create_info = create_info.next(&messenger_info as *const _ as *const _);
        }

        let mut handle: vk::Instance = null_mut();
        call!(vk::create_instance(&create_info, null(), &mut handle))?;

        #[allow(unused_mut)]
        let mut obj = Self {
            handle,
            #[cfg(debug_assertions)]
            debug_messenger: null_mut(),
        };

        #[cfg(debug_assertions)]
        {
            if let Some(func) = obj.get_proc_address::<vk::PFN_vkCreateDebugUtilsMessengerEXT>(
                "vkCreateDebugUtilsMessengerEXT",
            ) {
                call!(func(
                    obj.handle,
                    &messenger_info,
                    null(),
                    &mut obj.debug_messenger
                ))?;
            }
        }

        Ok(obj)
    }

    pub fn enumerate_extensions() -> Result<Vec<vk::ExtensionProperties>> {
        let mut count = 0u32;
        call!(vk::enumerate_instance_extension_properties(
            null(),
            &mut count,
            null_mut()
        ))?;

        let mut extensions = vec![Default::default(); count as usize];
        call!(vk::enumerate_instance_extension_properties(
            null(),
            &mut count,
            extensions.as_mut_ptr()
        ))?;

        Ok(extensions)
    }

    pub fn enumerate_layers() -> Result<Vec<vk::LayerProperties>> {
        let mut count = 0u32;
        call!(vk::enumerate_instance_layer_properties(
            &mut count,
            null_mut()
        ))?;

        let mut layers = vec![Default::default(); count as usize];
        call!(vk::enumerate_instance_layer_properties(
            &mut count,
            layers.as_mut_ptr()
        ))?;

        Ok(layers)
    }

    pub fn enumerate_physical_devices(&self) -> Result<Vec<PhysicalDevice>> {
        let mut count = 0u32;
        call!(vk::enumerate_physical_devices(
            self.handle,
            &mut count,
            null_mut()
        ))?;

        let mut devices = vec![null_mut(); count as usize];
        call!(vk::enumerate_physical_devices(
            self.handle,
            &mut count,
            devices.as_mut_ptr()
        ))?;

        devices
            .into_iter()
            .map(TryInto::<PhysicalDevice>::try_into)
            .collect()
    }

    pub fn get_proc_address<T>(&self, name: &str) -> T {
        use std::mem;

        assert!(mem::size_of::<T>() == mem::size_of::<vk::PFN_vkVoidFunction>());

        unsafe {
            let result =
                vk::get_instance_proc_addr(self.handle, CString::new(name).unwrap().as_ptr());
            mem::transmute_copy(&mem::ManuallyDrop::new(result))
        }
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            #[cfg(debug_assertions)]
            if !self.debug_messenger.is_null() {
                let func = self
                    .get_proc_address::<vk::PFN_vkDestroyDebugUtilsMessengerEXT>(
                        "vkDestroyDebugUtilsMessengerEXT",
                    )
                    .unwrap();

                func(self.handle, self.debug_messenger, null());
            }

            vk::destroy_instance(self.handle, null());
        }
    }
}

#[derive(magma_proc::Builder, Clone)]
#[builder(target = Instance)]
pub struct Builder {
    app_name: String,
    #[builder(skip)]
    app_version: u32,
    engine_name: String,
    #[builder(skip)]
    engine_version: u32,
    #[builder(default = vk::API_VERSION_1_0)]
    api_version: u32,
    extensions: Vec<String>,
    layers: Vec<String>,
}

impl Builder {
    pub fn app_version(mut self, major: u32, minor: u32, patch: u32) -> Self {
        self.app_version = vk::make_version(major, minor, patch);
        self
    }

    pub fn engine_version(mut self, major: u32, minor: u32, patch: u32) -> Self {
        self.engine_version = vk::make_version(major, minor, patch);
        self
    }
}
