use crate::vk;
use std::fmt;

pub fn to_string<T>(bytes: &[T]) -> String
where
    T: TryInto<u8> + Copy,
{
    let bytes: Vec<_> = bytes
        .iter()
        .copied()
        .filter_map(|x| x.try_into().ok())
        .take_while(|&x| x != 0)
        .collect();

    String::from_utf8_lossy(&bytes).to_string()
}

macro_rules! to_string {
    ($object:ident.$field:ident) => {
        impl fmt::Display for vk::$object {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}", to_string(&self.$field))
            }
        }
    };
}

to_string!(ExtensionProperties.extension_name);
to_string!(LayerProperties.layer_name);
to_string!(PhysicalDeviceProperties.device_name);
