extern crate voodoo as voo;

use std::time;
use std::ffi::{CStr, CString};
use voo::*;
use voo::Result as VdResult;


pub const ENABLE_VALIDATION_LAYERS: bool = false;

static REQUIRED_DEVICE_EXTENSIONS: &[&[u8]] = &[
    b"VK_KHR_swapchain\0",
];


/// Initializes and returns a new loader and instance.
fn init_instance() -> VdResult<Instance> {
    let app_name = CString::new("Hello!")?;
    let eng_name = CString::new("Engine")?;

    let app_info = ApplicationInfo::builder()
        .application_name(&app_name)
        .application_version((1, 0, 0))
        .engine_name(&eng_name)
        .engine_version((1, 0, 0))
        .api_version((1, 0, 0))
        .build();

    let loader = Loader::new()?;

    Instance::builder()
        .application_info(&app_info)
        .enabled_extensions(&loader.enumerate_instance_extension_properties()?)
        .build(loader)
}

/// Returns true if the specified physical device has the required features,
/// extensions, queue families and if the supported swap chain has the correct
/// presentation modes.
fn device_is_suitable(physical_device: &PhysicalDevice) -> VdResult<bool> {
    let reqd_exts: Vec<_> = (&REQUIRED_DEVICE_EXTENSIONS[..]).iter().map(|ext_name| {
        CStr::from_bytes_with_nul(ext_name).expect("invalid required extension name")
    }).collect();

    let extensions_supported = physical_device.verify_extension_support(&reqd_exts[..])?;
    Ok(extensions_supported)
}

/// Returns a physical device from the list of available physical devices if
/// it meets the criteria specified in the above function.
fn choose_physical_device(instance: &Instance)
        -> VdResult<PhysicalDevice> {
    let mut preferred_device = None;
    for device in instance.physical_devices()? {
        if device_is_suitable(&device)? {
            preferred_device = Some(device);
            break;
        }
    }
    if let Some(preferred_device) = preferred_device {
        Ok(preferred_device)
    } else {
        panic!("Failed to find a suitable device.");
    }
}

fn create_device(physical_device: PhysicalDevice)
        -> VdResult<Device> {
    let queue_priorities = [1.0];
    let queue_create_info = DeviceQueueCreateInfo::builder()
        .queue_family_index(0)
        .queue_priorities(&queue_priorities)
        .build();

    let features = PhysicalDeviceFeatures::builder()
        .sampler_anisotropy(true)
        .build();

    Device::builder()
        .queue_create_infos(&[queue_create_info.clone()])
        .enabled_extension_names(REQUIRED_DEVICE_EXTENSIONS)
        .enabled_features(&features)
        .build(physical_device)
}

fn create_test_buffers(device: &Device, flags: MemoryPropertyFlags)
        -> VdResult<(Vec<Buffer>, Vec<DeviceMemory>)> {
    let mut current_start;

    let mut buffers = Vec::with_capacity(16);
    let mut allocs = Vec::with_capacity(16);
    let mb16 = 1 << 20;

    for i in 0..11 {
        current_start = time::Instant::now();

        let buffer_bytes = (mb16 << i) as u64;
        let buffer = Buffer::builder()
            .size(buffer_bytes)
            .usage(BufferUsageFlags::UNIFORM_BUFFER)
            .sharing_mode(SharingMode::Exclusive)
            .build(device.clone())?;

        let memory_requirements = buffer.memory_requirements().clone();
        let memory_type_index = device.memory_type_index(memory_requirements.memory_type_bits(),
            flags)?;
        let buffer_memory_res = DeviceMemory::new(device.clone(), memory_requirements.size(),
            memory_type_index);

        let buffer_memory = match buffer_memory_res {
            Ok(bm) => bm,
            Err(_err) => {
                println!("Error creating buffer memory (probably oom).");
                continue;
            },
        };

        let duration = time::Instant::now() - current_start;

        buffer.bind_memory(&buffer_memory, 0)?;

        println!("buffer size: {}B; allocation time: {}.{:09}s",
            buffer_bytes, duration.as_secs(), duration.subsec_nanos());

        buffers.push(buffer);
        allocs.push(buffer_memory);
    }
    Ok((buffers, allocs))
}


fn test_alloc() -> VdResult<()> {
    let instance = init_instance()?;
    let physical_device = choose_physical_device(&instance)?;
    let device = create_device(physical_device)?;

    println!("Host:");
    let (_test_buffers, _test_buffer_allocs) = create_test_buffers(&device,
        MemoryPropertyFlags::HOST_VISIBLE)?;

    println!("Device:");
    let (_test_buffers, _test_buffer_allocs) = create_test_buffers(&device,
        MemoryPropertyFlags::DEVICE_LOCAL)?;

    Ok(())
}

fn main() {
    println!("Beginning buffer test.");
    test_alloc().unwrap();
}