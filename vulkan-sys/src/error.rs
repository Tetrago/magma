use crate::vk;
use std::fmt;

#[derive(Clone)]
pub struct VulkanError(pub vk::Result, pub Option<String>);

impl fmt::Debug for VulkanError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "VulkanError({})",
            vk::result_to_string(self.0).unwrap_or("Unknown")
        )?;

        if let Some(message) = &self.1 {
            write!(f, ": {}", message)?;
        }

        Ok(())
    }
}

impl fmt::Display for VulkanError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

pub type Result<T> = std::result::Result<T, VulkanError>;
