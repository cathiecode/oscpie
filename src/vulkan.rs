use anyhow::Result;
use log::{debug, log_enabled, trace};
use std::{default::Default, mem::forget, sync::Arc};
use tiny_skia::Pixmap;
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage,
        CopyBufferToImageInfo, PrimaryAutoCommandBuffer, PrimaryCommandBufferAbstract,
    },
    device::{
        physical::PhysicalDeviceType, Device, DeviceCreateInfo, DeviceExtensions, Queue,
        QueueCreateInfo, QueueFlags,
    },
    format::Format,
    image::{Image, ImageCreateInfo, ImageType, ImageUsage},
    instance::{
        debug::{
            DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger,
            DebugUtilsMessengerCallback, DebugUtilsMessengerCreateInfo,
        },
        Instance, InstanceCreateFlags, InstanceCreateInfo, InstanceExtensions,
    },
    memory::{
        allocator::{
            AllocationCreateInfo, FreeListAllocator, GenericMemoryAllocator,
            GenericMemoryAllocatorCreateInfo, MemoryTypeFilter,
        },
        MemoryProperties, MemoryPropertyFlags,
    },
    DeviceSize, VulkanLibrary,
};

use crate::openvr::{CompositorInterface, Handle};

type StandardLiteMemoryAllocator = GenericMemoryAllocator<FreeListAllocator>;

trait StandardLiteMemoryAllocatorExt {
    /// Creates a new `StandardLiteMemoryAllocator` with default configuration.
    fn new_default_lite(device: Arc<Device>) -> Self;
}

impl StandardLiteMemoryAllocatorExt for StandardLiteMemoryAllocator {
    /// Creates a new `StandardMemoryAllocator` with default configuration.
    fn new_default_lite(device: Arc<Device>) -> Self {
        let MemoryProperties {
            memory_types,
            memory_heaps,
            ..
        } = device.physical_device().memory_properties();

        let mut block_sizes = vec![0; memory_types.len()];
        let mut memory_type_bits = u32::MAX;

        for (index, memory_type) in memory_types.iter().enumerate() {
            const LARGE_HEAP_THRESHOLD: DeviceSize = 1024 * 1024 * 1024;

            let heap_size = memory_heaps[memory_type.heap_index as usize].size;

            block_sizes[index] = if heap_size >= LARGE_HEAP_THRESHOLD {
                4 * 1024 * 1024
            } else {
                2 * 1024 * 1024
            };

            if memory_type.property_flags.intersects(
                MemoryPropertyFlags::LAZILY_ALLOCATED
                    | MemoryPropertyFlags::PROTECTED
                    | MemoryPropertyFlags::DEVICE_COHERENT
                    | MemoryPropertyFlags::RDMA_CAPABLE,
            ) {
                // VUID-VkMemoryAllocateInfo-memoryTypeIndex-01872
                // VUID-vkAllocateMemory-deviceCoherentMemory-02790
                // Lazily allocated memory would just cause problems for suballocation in general.
                memory_type_bits &= !(1 << index);
            }
        }

        let create_info = GenericMemoryAllocatorCreateInfo {
            block_sizes: &block_sizes,
            memory_type_bits,
            ..Default::default()
        };

        Self::new(device, create_info)
    }
}

pub struct ImageUploader {
    upload_buffer: Subbuffer<[u8]>,
    queue: Arc<Queue>,
    image: Arc<Image>,
    command_buffer: Arc<PrimaryAutoCommandBuffer>,
    pixmap: *const Pixmap,
}

impl ImageUploader {
    pub fn new(pixmap: &Pixmap, compositor_interface: Handle<CompositorInterface>) -> Result<Self> {
        let width = pixmap.width();
        let height = pixmap.height();
        let extent = [width, height, 1];

        let library = VulkanLibrary::new().unwrap();

        let instance_flags_request =
            compositor_interface.get_vulkan_instance_extensions_required()?;
        let mut instance_extensions = InstanceExtensions::from_iter(
            instance_flags_request
                .iter()
                .map(std::string::String::as_str),
        );

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
                enabled_layers,
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
                ));
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

        debug!("Vulkan device extensions: {device_extensions:?}");

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

        let memory_allocator = Arc::new(StandardLiteMemoryAllocator::new_default_lite(
            device.clone(),
        ));

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
            DeviceSize::from(width * height * 4),
        )
        .unwrap();

        let image = Image::new(
            memory_allocator,
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_SRGB,
                extent,
                usage: ImageUsage::TRANSFER_SRC | ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap();

        let mut uploads = AutoCommandBufferBuilder::primary(
            command_buffer_allocator.clone(),
            queue.queue_family_index(),
            CommandBufferUsage::MultipleSubmit,
        )
        .unwrap();

        {
            uploads
                .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                    upload_buffer.clone(),
                    image.clone(),
                ))
                .unwrap();
        }

        let command_buffer = uploads.build().unwrap();

        Ok(ImageUploader {
            queue,
            upload_buffer,
            image,
            command_buffer,
            pixmap: std::ptr::from_ref::<Pixmap>(pixmap),
        })
    }

    pub fn upload(&mut self, pixmap: &Pixmap) -> Arc<Image> {
        assert!(
            std::ptr::from_ref::<Pixmap>(pixmap) == self.pixmap,
            "pixmap mismatch"
        );

        {
            let mut writer = self.upload_buffer.write().unwrap();
            writer.copy_from_slice(pixmap.data());
        }

        let _ = self
            .command_buffer
            .clone()
            .execute(self.queue.clone())
            .unwrap();

        self.image.clone()
    }

    pub fn queue(&self) -> &Queue {
        self.queue.as_ref()
    }
}
