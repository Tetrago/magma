use std::fmt;
use vulkan_sys::VulkanError;

pub enum Error {
    Vulkan(VulkanError),
    MissingInstanceExtensions(Vec<String>),
    MissingInstanceLayers(Vec<String>),
    InvalidQueueCount {
        requested: u32,
        index: u32,
        available: u32,
    },
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Vulkan(err) => write!(f, "{}", err),
            Error::MissingInstanceExtensions(extensions) => {
                write!(f, "Missing instance extensions:")?;

                for extension in extensions {
                    write!(f, "\n- {}", extension)?;
                }

                Ok(())
            }
            Error::MissingInstanceLayers(extensions) => {
                write!(f, "Missing instance layers:")?;

                for extension in extensions {
                    write!(f, "\n- {}", extension)?;
                }

                Ok(())
            }
            Error::InvalidQueueCount {
                requested,
                index,
                available,
            } => write!(
                f,
                "{} queues requested for queue family ({}) that only supports {}",
                requested, index, available
            ),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl From<VulkanError> for Error {
    fn from(value: VulkanError) -> Self {
        Self::Vulkan(value)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
