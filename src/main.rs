use anyhow::Result;
use log::*;

use tiny_skia::*;
use vulkano::{
    device::DeviceExtensions,
    image::Image,
    instance::{Instance, InstanceExtensions},
    Handle, VulkanObject,
};
mod openvr;

mod vulkan {
    use anyhow::Result;
    use log::*;
    use std::{
        default::Default,
        fs::File,
        io::{BufReader, BufWriter},
        mem::forget,
        path::Path,
        sync::Arc,
    };
    use tiny_skia::{Pixmap, PixmapMut};
    use vulkano::{
        buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer},
        command_buffer::{
            allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
            CommandBufferUsage, CopyBufferToImageInfo, CopyImageToBufferInfo,
            PrimaryCommandBufferAbstract, RenderPassBeginInfo, SubpassBeginInfo, SubpassContents,
        },
        device::{
            physical::PhysicalDeviceType, Device, DeviceCreateInfo, DeviceExtensions, Queue,
            QueueCreateInfo, QueueFlags,
        },
        format::Format,
        image::{
            sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo},
            view::ImageView,
            Image, ImageCreateInfo, ImageLayout, ImageType, ImageUsage,
        },
        instance::{
            debug::{
                DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger,
                DebugUtilsMessengerCallback, DebugUtilsMessengerCreateInfo,
            },
            Instance, InstanceCreateFlags, InstanceCreateInfo, InstanceExtensions,
        },
        memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
        pipeline::{
            graphics::{
                color_blend::{ColorBlendAttachmentState, ColorBlendState},
                input_assembly::InputAssemblyState,
                multisample::MultisampleState,
                rasterization::RasterizationState,
                vertex_input::{Vertex, VertexDefinition},
                viewport::{Viewport, ViewportState},
                GraphicsPipelineCreateInfo,
            },
            layout::PipelineDescriptorSetLayoutCreateInfo,
            GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
        },
        render_pass::{Framebuffer, FramebufferCreateInfo, Subpass},
        swapchain::Surface,
        sync::GpuFuture,
        DeviceSize, VulkanLibrary,
    };

    use crate::openvr::CompositorInterface;

    fn script() {
        let library = VulkanLibrary::new().unwrap();

        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                ..Default::default()
            },
        )
        .unwrap();

        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()
            .unwrap()
            // No need for swapchain extension support.
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .position(|q| q.queue_flags.intersects(QueueFlags::GRAPHICS))
                    .map(|i| (p, i as u32))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
                _ => 5,
            })
            .expect("no suitable physical device found");

        debug!(
            "Using device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );

        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .unwrap();

        let queue = queues.next().unwrap();

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let mut uploads = AutoCommandBufferBuilder::primary(
            command_buffer_allocator.clone(),
            queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        // ----------------- テクスチャの用意
        let texture = {
            let png_bytes = include_bytes!("image_img.png").as_slice();
            let decoder = png::Decoder::new(png_bytes);
            let mut reader = decoder.read_info().unwrap();
            let info = reader.info();
            let extent = [info.width, info.height, 1];

            // ----------------- GPU側のバッファ
            let upload_buffer: vulkano::buffer::Subbuffer<[u8]> = Buffer::new_slice(
                memory_allocator.clone(),
                BufferCreateInfo {
                    usage: BufferUsage::TRANSFER_SRC,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    memory_type_filter: MemoryTypeFilter::PREFER_HOST
                        | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                    ..Default::default()
                },
                (info.width * info.height * 4) as DeviceSize,
            )
            .unwrap();

            // write: ロックしてBufferを晒す
            reader
                .next_frame(&mut upload_buffer.write().unwrap())
                .unwrap();

            let image = Image::new(
                memory_allocator,
                ImageCreateInfo {
                    image_type: ImageType::Dim2d,
                    format: Format::R8G8B8A8_SRGB,
                    extent,
                    usage: ImageUsage::TRANSFER_SRC
                        | ImageUsage::TRANSFER_DST
                        | ImageUsage::SAMPLED,
                    initial_layout: ImageLayout::TransferSrcOptimal,
                    ..Default::default()
                },
                AllocationCreateInfo::default(),
            )
            .unwrap();

            uploads
                .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                    upload_buffer,
                    image.clone(),
                ))
                .unwrap();

            ImageView::new_default(image).unwrap()
        };

        let _ = uploads.build().unwrap().execute(queue.clone()).unwrap();
    }

    pub struct ImageUploader {
        upload_buffer: Subbuffer<[u8]>,
        command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
        instance: Arc<Instance>,
        device: Arc<Device>,
        queue: Arc<Queue>,
        image: Arc<Image>,
        pixmap: *const Pixmap,
    }

    impl ImageUploader {
        pub fn new(pixmap: &Pixmap, compositor_interface: &CompositorInterface) -> Result<Self> {
            let width = pixmap.width();
            let height = pixmap.height();
            let extent = [width, height, 1];

            let library = VulkanLibrary::new().unwrap();

            let instance_flags_request =
                compositor_interface.get_vulkan_instance_extensions_required()?;
            let mut instance_extensions =
                InstanceExtensions::from_iter(instance_flags_request.iter().map(|s| s.as_str()));

            instance_extensions.ext_debug_utils = true;

            let mut enabled_layers = Vec::new();

            if log_enabled!(log::Level::Trace) {
                debug!("List of Vulkan layers available to use:");
                let layers = library.layer_properties().unwrap();
                for l in layers {
                    debug!("\t{}", l.name());
                }

                enabled_layers.push("VK_LAYER_KHRONOS_validation".to_owned());
            }

            let instance = Instance::new(
                library,
                InstanceCreateInfo {
                    flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                    enabled_extensions: instance_extensions,
                    enabled_layers: enabled_layers,
                    ..Default::default()
                },
            )
            .unwrap();

            if log_enabled!(log::Level::Trace) {
                unsafe {
                    forget(DebugUtilsMessenger::new(
                        instance.clone(),
                        DebugUtilsMessengerCreateInfo {
                            message_severity: DebugUtilsMessageSeverity::ERROR
                                | DebugUtilsMessageSeverity::WARNING
                                | DebugUtilsMessageSeverity::INFO
                                | DebugUtilsMessageSeverity::VERBOSE,
                            message_type: DebugUtilsMessageType::GENERAL
                                | DebugUtilsMessageType::VALIDATION
                                | DebugUtilsMessageType::PERFORMANCE,
                            ..DebugUtilsMessengerCreateInfo::user_callback(
                                DebugUtilsMessengerCallback::new(
                                    |message_severity, message_type, callback_data| {
                                        let severity = if message_severity
                                            .intersects(DebugUtilsMessageSeverity::ERROR)
                                        {
                                            "error"
                                        } else if message_severity
                                            .intersects(DebugUtilsMessageSeverity::WARNING)
                                        {
                                            "warning"
                                        } else if message_severity
                                            .intersects(DebugUtilsMessageSeverity::INFO)
                                        {
                                            "information"
                                        } else if message_severity
                                            .intersects(DebugUtilsMessageSeverity::VERBOSE)
                                        {
                                            "verbose"
                                        } else {
                                            panic!("no-impl");
                                        };

                                        let ty = if message_type
                                            .intersects(DebugUtilsMessageType::GENERAL)
                                        {
                                            "general"
                                        } else if message_type
                                            .intersects(DebugUtilsMessageType::VALIDATION)
                                        {
                                            "validation"
                                        } else if message_type
                                            .intersects(DebugUtilsMessageType::PERFORMANCE)
                                        {
                                            "performance"
                                        } else {
                                            panic!("no-impl");
                                        };

                                        trace!(
                                            "{} {} {}: {}",
                                            callback_data.message_id_name.unwrap_or("unknown"),
                                            ty,
                                            severity,
                                            callback_data.message
                                        );
                                    },
                                ),
                            )
                        },
                    ))
                };
            }

            let (physical_device, queue_family_index) = instance
                .enumerate_physical_devices()
                .unwrap()
                // No need for swapchain extension support.
                .filter_map(|p| {
                    p.queue_family_properties()
                        .iter()
                        .position(|q| q.queue_flags.intersects(QueueFlags::GRAPHICS))
                        .map(|i| (p, i as u32))
                })
                .min_by_key(|(p, _)| match p.properties().device_type {
                    PhysicalDeviceType::DiscreteGpu => 0,
                    PhysicalDeviceType::IntegratedGpu => 1,
                    PhysicalDeviceType::VirtualGpu => 2,
                    PhysicalDeviceType::Cpu => 3,
                    PhysicalDeviceType::Other => 4,
                    _ => 5,
                })
                .expect("no suitable physical device found");

            debug!(
                "Using device: {} (type: {:?})",
                physical_device.properties().device_name,
                physical_device.properties().device_type,
            );

            let device_extensions_request =
                compositor_interface.get_vulkan_device_extensions_required(&physical_device)?;
            let device_extensions = DeviceExtensions::from_iter(
                device_extensions_request
                    .iter()
                    .map(|s: &String| s.as_str()),
            );

            debug!("Vulkan device extensions: {:?}", device_extensions);

            let (device, mut queues) = Device::new(
                physical_device.clone(),
                DeviceCreateInfo {
                    queue_create_infos: vec![QueueCreateInfo {
                        queue_family_index,
                        ..Default::default()
                    }],
                    enabled_extensions: device_extensions,
                    ..Default::default()
                },
            )
            .unwrap();

            let queue = queues.next().unwrap();

            let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

            let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
                device.clone(),
                Default::default(),
            ));

            let upload_buffer: vulkano::buffer::Subbuffer<[u8]> = Buffer::new_slice(
                memory_allocator.clone(),
                BufferCreateInfo {
                    usage: BufferUsage::TRANSFER_SRC,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    memory_type_filter: MemoryTypeFilter::PREFER_HOST
                        | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                    ..Default::default()
                },
                (width * height * 4) as DeviceSize,
            )
            .unwrap();

            let image = Image::new(
                memory_allocator,
                ImageCreateInfo {
                    image_type: ImageType::Dim2d,
                    format: Format::R8G8B8A8_SRGB,
                    extent,
                    usage: ImageUsage::TRANSFER_SRC
                        | ImageUsage::TRANSFER_DST
                        | ImageUsage::SAMPLED,
                    ..Default::default()
                },
                AllocationCreateInfo::default(),
            )
            .unwrap();

            Ok(ImageUploader {
                instance,
                device,
                queue,
                // memory_allocator,
                command_buffer_allocator,
                upload_buffer,
                image,
                pixmap: pixmap as *const Pixmap,
            })
        }

        pub fn upload(&mut self, pixmap: &Pixmap) -> Arc<Image> {
            if pixmap as *const Pixmap != self.pixmap {
                panic!("pixmap mismatch");
            }

            let mut uploads = AutoCommandBufferBuilder::primary(
                self.command_buffer_allocator.clone(),
                self.queue.queue_family_index(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap();

            {
                let mut writer = self.upload_buffer.write().unwrap();

                writer.copy_from_slice(pixmap.data());

                uploads
                    .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                        self.upload_buffer.clone(),
                        self.image.clone(),
                    ))
                    .unwrap();
            }

            let _ = uploads
                .build()
                .unwrap()
                .execute(self.queue.clone())
                .unwrap();

            return self.image.clone();
        }

        pub fn queue<'a>(&'a self) -> &'a Queue {
            self.queue.as_ref()
        }
    }
}

struct App {

}

fn paint(pixmap: &mut Pixmap) {
    let mut paint = Paint::default();
    paint.set_color_rgba8(0, 127, 0, 200);
    paint.anti_alias = true;

    let path = {
        let mut pb = PathBuilder::new();
        const RADIUS: f32 = 250.0;
        const CENTER: f32 = 250.0;
        pb.move_to(CENTER + RADIUS, CENTER);
        for i in 1..8 {
            let a = 2.6927937 * i as f32;
            pb.line_to(CENTER + RADIUS * a.cos(), CENTER + RADIUS * a.sin());
        }
        pb.finish().unwrap()
    };

    let mut stroke = Stroke::default();
    stroke.width = 6.0;
    stroke.line_cap = LineCap::Round;
    stroke.dash = StrokeDash::new(vec![20.0, 40.0], 0.0);

    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
}

fn app() -> Result<()> {
    let openvr = openvr::OpenVR::new(openvr::EVRApplicationType::Overlay)?;
    let overlay_interface = openvr.overlay()?;
    let compositor = openvr.compositor()?;

    let overlay = overlay_interface.create("oscpie_overlay_2", "OSCPie Overlay")?;

    overlay.show()?;

    let mut pixmap: Pixmap = Pixmap::new(512, 512).unwrap();
    let mut uploader = vulkan::ImageUploader::new(&pixmap, &compositor)?;

    loop {
        paint(&mut pixmap);

        let image = uploader.upload(&pixmap);

        let texture_handle = openvr::TextureHandle::Vulkan(image.as_ref(), uploader.queue());

        let mut texture = openvr::Texture {
            handle: texture_handle,
            texture_type: openvr::TextureType::Vulkan,
            color_space: openvr::ColorSpace::Auto,
        };

        // TODO: transfer data via Vulkan, OpenGL, or DirectX

        debug!("texture: {:?}", texture);
        overlay.set_overlay_texture(&mut texture)?;

        std::thread::sleep(std::time::Duration::from_millis(1000));
    }

    overlay.hide()?;

    Ok(())
}

fn main() {
    env_logger::init();
    app().unwrap();
    // vulkan_image::main();
}
