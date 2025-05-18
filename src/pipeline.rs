use crate::Device;
use crate::RenderPass;
use crate::Result;
use std::cell::RefCell;
use std::ffi::CString;
use std::ptr::null;
use std::ptr::null_mut;
use std::rc::Rc;
use std::sync::Arc;
use vulkan_sys::bool_cast;
use vulkan_sys::call;
use vulkan_sys::vk;

#[derive(magma_proc::Object)]
#[object(builder = Builder)]
pub struct Pipeline {
    #[object]
    handle: vk::Pipeline,
    layout: vk::PipelineLayout,
    device: Arc<Device>,
    _render_pass: Arc<RenderPass>,
}

impl Pipeline {
    fn new(builder: Builder) -> Result<Self> {
        let device = builder.device.unwrap();
        let render_pass = builder.render_pass.unwrap();

        let dynamic_state = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_state_count(builder.dynamic_states.len() as u32)
            .dynamic_states(builder.dynamic_states.as_ptr());

        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_description_count(builder.bindings.len() as u32)
            .vertex_binding_descriptions(builder.bindings.as_ptr())
            .vertex_attribute_description_count(builder.attributes.len() as u32)
            .vertex_attribute_descriptions(builder.attributes.as_ptr());

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(builder.topology)
            .primitive_restart_enable(bool_cast!(builder.primitive_restart));

        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        let rasterization_state = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(bool_cast!(builder.depth_clamp))
            .rasterizer_discard_enable(bool_cast!(builder.rasterizer_discard))
            .polygon_mode(builder.polygon_mode)
            .line_width(builder.line_width)
            .cull_mode(builder.cull_mode)
            .front_face(builder.front_face)
            .depth_bias_enable(vk::FALSE);

        let multisampling_state = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(vk::FALSE)
            .rasterization_samples(vk::SAMPLE_COUNT_1_BIT);

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(
                vk::COLOR_COMPONENT_R_BIT
                    | vk::COLOR_COMPONENT_G_BIT
                    | vk::COLOR_COMPONENT_B_BIT
                    | vk::COLOR_COMPONENT_A_BIT,
            )
            .blend_enable(vk::FALSE);

        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(vk::FALSE)
            .attachment_count(1)
            .attachments(&color_blend_attachment);

        let layout = {
            let mut handle: vk::PipelineLayout = null_mut();

            let create_info = vk::PipelineLayoutCreateInfo::default();
            call!(vk::create_pipeline_layout(
                device.handle(),
                &create_info,
                null(),
                &mut handle
            ))?;

            handle
        };

        let entrypoint = CString::new("main").unwrap();

        let stages = builder
            .modules
            .borrow()
            .iter()
            .map(|(stage, module)| {
                Ok(vk::PipelineShaderStageCreateInfo::default()
                    .stage(*stage)
                    .module(module.as_ref()?.handle)
                    .name(entrypoint.as_ptr()))
            })
            .collect::<Result<Vec<_>>>()?;

        let create_info = vk::GraphicsPipelineCreateInfo::default()
            .stage_count(stages.len() as u32)
            .stages(stages.as_ptr())
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisampling_state)
            .color_blend_state(&color_blend_state)
            .dynamic_state(&dynamic_state)
            .layout(layout)
            .render_pass(render_pass.handle())
            .subpass(builder.subpass);

        let mut handle: vk::Pipeline = null_mut();
        call!(vk::create_graphics_pipelines(
            device.handle(),
            null_mut(),
            1,
            &create_info,
            null(),
            &mut handle
        ))?;

        Ok(Self {
            handle,
            layout,
            device,
            _render_pass: render_pass,
        })
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            vk::destroy_pipeline_layout(self.device.handle(), self.layout, null());
            vk::destroy_pipeline(self.device.handle(), self.handle, null());
        }
    }
}

struct ShaderModule {
    device: Arc<Device>,
    handle: vk::ShaderModule,
}

impl ShaderModule {
    fn new(device: Arc<Device>, code: &[u8]) -> Result<Self> {
        let create_info = vk::ShaderModuleCreateInfo::default()
            .code(code.as_ptr() as *const _)
            .code_size(code.len());

        let mut handle: vk::ShaderModule = null_mut();
        call!(vk::create_shader_module(
            device.handle(),
            &create_info,
            null(),
            &mut handle
        ))?;

        Ok(Self { device, handle })
    }
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        unsafe {
            vk::destroy_shader_module(self.device.handle(), self.handle, null());
        }
    }
}

#[derive(magma_proc::Builder, Clone)]
#[builder(target = Pipeline)]
pub struct Builder {
    #[builder(required)]
    device: Option<Arc<Device>>,
    #[builder(required)]
    render_pass: Option<Arc<RenderPass>>,
    modules: Rc<RefCell<Vec<(vk::Enum, Result<ShaderModule>)>>>,
    dynamic_states: Vec<vk::Enum>,
    #[builder(default = vk::PRIMITIVE_TOPOLOGY_TRIANGLE_LIST)]
    topology: vk::Enum,
    primitive_restart: bool,
    depth_clamp: bool,
    rasterizer_discard: bool,
    #[builder(default = vk::POLYGON_MODE_FILL)]
    polygon_mode: vk::Enum,
    #[builder(default = 1.0)]
    line_width: f32,
    #[builder(default = vk::CULL_MODE_BACK_BIT)]
    cull_mode: vk::Enum,
    #[builder(default = vk::FRONT_FACE_CLOCKWISE)]
    front_face: vk::Enum,
    #[builder(default = 0)]
    subpass: u32,
    #[builder(skip)]
    bindings: Vec<vk::VertexInputBindingDescription>,
    #[builder(skip)]
    attributes: Vec<vk::VertexInputAttributeDescription>,
}

impl Builder {
    pub fn compile_shader(self, stage: vk::Enum, code: &[u8]) -> Self {
        self.modules.borrow_mut().push((
            stage,
            ShaderModule::new(
                self.device
                    .as_ref()
                    .expect("Device should be supplied prior to compiling shaders")
                    .clone(),
                code,
            ),
        ));

        self
    }

    pub fn binding(mut self, binding: u32, stride: usize) -> Self {
        self.bindings.push(
            vk::VertexInputBindingDescription::default()
                .binding(binding)
                .stride(stride as u32)
                .input_rate(vk::VERTEX_INPUT_RATE_VERTEX),
        );

        self
    }

    pub fn instance_binding(mut self, binding: u32, stride: usize) -> Self {
        self.bindings.push(
            vk::VertexInputBindingDescription::default()
                .binding(binding)
                .stride(stride as u32)
                .input_rate(vk::VERTEX_INPUT_RATE_INSTANCE),
        );

        self
    }

    pub fn attribute(
        mut self,
        binding: u32,
        location: u32,
        format: vk::Format,
        offset: usize,
    ) -> Self {
        self.attributes.push(
            vk::VertexInputAttributeDescription::default()
                .binding(binding)
                .location(location)
                .format(format)
                .offset(offset as u32),
        );

        self
    }
}
