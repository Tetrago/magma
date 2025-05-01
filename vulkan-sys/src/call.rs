#[macro_export]
macro_rules! call {
    ($expr:expr) => {unsafe {
        let result = $expr;
        if result == ::vulkan_sys::vk::SUCCESS {
            ::vulkan_sys::Result::Ok(())
        } else {
            ::vulkan_sys::Result::Err(::vulkan_sys::VulkanError(result, Some(stringify!($expr).to_owned())))
        }
    }};
    ($expr:expr, $message:literal) => {unsafe {
        let result = $expr;
        if result == ::vulkan_sys::vk::SUCCESS {
            ::vulkan_sys::Result::Ok(())
        } else {
            ::vulkan_sys::Result::Err(::vulkan_sys::VulkanError(result, Some($message.to_owned())))
        }
    }};
    ($expr:expr, $fmt:literal, $($arg:tt)+) => {unsafe {
        let result = $expr;
        if result == ::vulkan_sys::vk::SUCCESS {
            ::vulkan_sys::Result::Ok(())
        } else {
            ::vulkan_sys::Result::Err(::vulkan_sys::VulkanError(result, Some(format!($fmt, $($arg)+))))
        }
    }};
}
