use crate::Result;
use vma_sys::vma;
use vulkan_sys::vk;

#[derive(magma_proc::Object)]
#[object(builder = Builder)]
pub struct Buffer {
    #[object]
    handle: vk::Buffer,
    allocation: vma::Allocation,
    info: vma::AllocationInfo,
}

impl Buffer {
    fn new(builder: Builder) -> Result<Self> {
        unimplemented!()
    }
}

#[derive(magma_proc::Builder, Clone)]
#[builder(target = Buffer)]
pub struct Builder {}
