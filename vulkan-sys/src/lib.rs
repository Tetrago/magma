#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused)]
#![allow(unsafe_op_in_unsafe_fn)]

mod call;
mod error;
mod to_string;

pub use error::Result;
pub use error::VulkanError;
pub use to_string::to_string;

pub mod vk {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

    pub const fn make_version(major: u32, minor: u32, patch: u32) -> u32 {
        (major << 22) | (minor << 12) | patch
    }

    pub const fn make_api_version(variant: u32, major: u32, minor: u32, patch: u32) -> u32 {
        (variant << 29) | (major << 22) | (minor << 12) | patch
    }

    pub const API_VERSION_1_0: u32 = make_api_version(0, 1, 0, 0);
    pub const API_VERSION_1_1: u32 = make_api_version(0, 1, 1, 0);
    pub const API_VERSION_1_2: u32 = make_api_version(0, 1, 2, 0);

    pub type Enum = u32;
    pub type EnumFlags = u32;
}

pub mod internal {
    mod internal {
        use super::super::vk::*;
        include!(concat!(env!("OUT_DIR"), "/internal.rs"));
    }

    pub use internal::CommandBufferInternal as CommandBuffer;
}

#[macro_export]
macro_rules! bool_cast {
    ($expr:expr) => {
        if $expr {
            ::vulkan_sys::vk::TRUE
        } else {
            ::vulkan_sys::vk::FALSE
        }
    };
}
