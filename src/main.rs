extern crate voodoo as voo;

use std::time;
use std::ffi::{CStr, CString};
use voo::*;
use voo::Result as VooResult;
use voodoo_winit::winit::{EventsLoop, WindowBuilder, Window};

// #[cfg(debug_assertions)]
// pub const ENABLE_VALIDATION_LAYERS: bool = true;
// #[cfg(not(debug_assertions))]
pub const ENABLE_VALIDATION_LAYERS: bool = false;

// static REQUIRED_INSTANCE_EXTENSIONS: &[&[u8]] = &[
//     b"VK_KHR_surface\0",
//     b"VK_KHR_win32_surface\0",
// ];

static REQUIRED_DEVICE_EXTENSIONS: &[&[u8]] = &[
    b"VK_KHR_swapchain\0",
];


/// Initializes the window and event loop.
fn init_window() -> (Window, EventsLoop) {
    let events_loop = EventsLoop::new();
    let window = WindowBuilder::new()
        .with_title("Voodoo - Hello Triangle")
        .build(&events_loop).unwrap();
    (window, events_loop)
}

/// Returns the list of layer names to be enabled.
fn enabled_layer_names<'ln>(loader: &Loader)
        -> Vec<&'ln CStr> {
    if ENABLE_VALIDATION_LAYERS && !loader.check_validation_layer_support() {
        panic!("Unable to enable validation layers.");
    }
    if ENABLE_VALIDATION_LAYERS {
         (loader.validation_layer_names()).iter().map(|lyr_name|
            unsafe { CStr::from_ptr(lyr_name.as_ptr() as *const i8) }).collect()
    } else {
        Vec::new()
    }
}

/// Initializes a loader and returns a new instance.
fn init_instance() -> VooResult<Instance> {
    let app_name = CString::new("Hello Triangle")?;
    let eng_name = CString::new("None")?;

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
        .enabled_layer_names(enabled_layer_names(&loader).as_slice())
        .enabled_extensions(loader.instance_extensions().as_slice())
        .build(loader, ENABLE_VALIDATION_LAYERS)
}

/// Returns true if the specified physical device has the required features,
/// extensions, queue families and if the supported swap chain has the correct
/// presentation modes.
fn device_is_suitable(_instance: &Instance, surface: &SurfaceKhr,
        physical_device: &PhysicalDevice, queue_family_flags: QueueFlags) -> VooResult<bool> {
    let device_features = physical_device.features()?;

    let reqd_exts: Vec<_> = (&REQUIRED_DEVICE_EXTENSIONS[..]).iter().map(|ext_name| {
        CStr::from_bytes_with_nul(ext_name).expect("invalid required extension name")
    }).collect();

    let extensions_supported = physical_device.verify_extensions_support(&reqd_exts[..])?;

    let mut swap_chain_adequate = false;
    if extensions_supported {
        let swap_chain_details = SwapchainSupportDetails::new(surface,
            &physical_device)?;
        swap_chain_adequate = !swap_chain_details.formats.is_empty() &&
            !swap_chain_details.present_modes.is_empty()
    }

    let queue_family_indices = queue::queue_families(surface,
        &physical_device, queue_family_flags)?;

    Ok(queue_family_indices.is_complete() &&
        extensions_supported &&
        swap_chain_adequate &&
        device_features.sampler_anisotropy())
}

/// Returns a physical device from the list of available physical devices if
/// it meets the criteria specified in the above function.
fn choose_physical_device(instance: &Instance, surface: &SurfaceKhr,
        queue_family_flags: QueueFlags) -> VooResult<PhysicalDevice> {
    let mut preferred_device = None;
    for device in instance.physical_devices() {
        if device_is_suitable(instance, surface, &device, queue_family_flags)? {
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

fn create_device(_instance: Instance, surface: &SurfaceKhr, physical_device: PhysicalDevice,
        queue_familiy_flags: QueueFlags) -> VooResult<Device> {
    let queue_family_idx = queue::queue_families(surface,
        &physical_device, queue_familiy_flags)?.family_idxs()[0] as u32;

    let queue_priorities = [1.0];
    let queue_create_info = DeviceQueueCreateInfo::builder()
        .queue_family_index(queue_family_idx)
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
        -> VooResult<(Vec<Buffer>, Vec<DeviceMemory>)> {
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


fn test_alloc() -> VooResult<()> {

    let instance = init_instance()?;
    let (window, _events_loop) = init_window();
    let surface = voodoo_winit::create_surface(instance.clone(), &window)?;
    let queue_family_flags = QueueFlags::GRAPHICS;
    let physical_device = choose_physical_device(&instance, &surface,
        queue_family_flags)?;
    let device = create_device(instance.clone(), &surface, physical_device,
        queue_family_flags)?;

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