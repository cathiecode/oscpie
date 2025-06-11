pub mod input;

use crate::prelude::*;
use anyhow::{anyhow, Result};
use log::{debug, trace};

use openvr_sys::{self as sys, VkDevice_T, VkInstance_T, VkPhysicalDevice_T, VkQueue_T};
use std::{
    ffi::{c_void, CStr},
    path::PathBuf,
    rc::Rc,
};
use vulkano::{
    device::{DeviceOwned, Queue},
    image::Image,
    Handle as _, VulkanObject,
};

#[derive(Debug, Clone, Copy)]
pub enum TextureType {
    Vulkan = 2,
}

#[derive(Debug, Clone, Copy)]
pub enum ColorSpace {
    Auto = 0,
    Gamma = 1,
    Linear = 2,
}

#[derive(Debug, Clone, Copy)]
pub enum TrackingUniverseOrigin {
    Standing = 0,
    Seated = 1,
    RawAndUncalibrated = 2,
}

#[derive(Debug)]
pub enum TextureHandle<'a> {
    Vulkan(&'a Image, &'a Queue),
    OpenGL(&'a mut c_void),
}

#[derive(Debug)]
pub struct Texture<'a> {
    pub handle: TextureHandle<'a>,
    pub texture_type: TextureType,
    pub color_space: ColorSpace,
}

#[derive(Debug, Clone, Copy)]
pub enum EVRApplicationType {
    Other = 0,
    Scene = 1,
    Overlay = 2,
    Background = 3,
    Utility = 4,
    VRMonitor = 5,
    SteamWatchdog = 6,
    Bootstrapper = 7,
    WebHelper = 8,
    OpenXRInstance = 9,
    OpenXRScene = 10,
    OpenXROverlay = 11,
    Prism = 12,
    RoomView = 13,
    Max = 14,
}

fn get_interface<T>(interface_version: &[u8]) -> Result<&T> {
    let mut interface_version_str: Vec<u8> = Vec::with_capacity(8 + interface_version.len());
    interface_version_str.extend_from_slice(b"FnTable:".as_slice());
    interface_version_str.extend_from_slice(interface_version);

    let mut error = sys::EVRInitError_VRInitError_None;

    let result = unsafe {
        sys::VR_GetGenericInterface(interface_version_str.as_ptr().cast::<i8>(), &mut error)
    };

    if error != sys::EVRInitError_VRInitError_None {
        return Err(anyhow::anyhow!("Failed to get interface: {}", error));
    }

    Ok(unsafe { &*(result as *const T) })
}

#[derive(Debug, Clone)]
struct CastRc<T>
where
    T: 'static,
{
    _rc: Rc<OpenVr>,
    value: &'static T,
}

impl<T> CastRc<T> {
    unsafe fn new(_rc: Rc<OpenVr>, value: &'static T) -> Self {
        CastRc { _rc, value }
    }

    fn get(&self) -> &T {
        self.value
    }
}

#[derive(Debug)]
pub struct OpenVr {
    openvr: isize,
}

#[derive(Clone)]
pub struct Handle<T>(Rc<T>);

impl Handle<OpenVr> {
    pub fn new(application_type: EVRApplicationType) -> Result<Self> {
        let mut error: openvr_sys::EVRInitError = openvr_sys::EVRInitError_VRInitError_None;

        let openvr = unsafe { openvr_sys::VR_InitInternal(&mut error, application_type as i32) };

        if error != sys::EVRInitError_VRInitError_None {
            return Err(anyhow::anyhow!("Failed to initialize OpenVR: {}", error));
        }

        Ok(Self(Rc::new(OpenVr { openvr })))
    }

    pub fn overlay(&self) -> Result<Handle<OverlayInterface>> {
        let sys = get_interface::<sys::VR_IVROverlay_FnTable>(sys::IVROverlay_Version)?;

        Ok(Handle(Rc::new(OverlayInterface {
            sys: unsafe { CastRc::new(self.0.clone(), sys) },
        })))
    }

    pub fn system(&self) -> Result<Handle<SystemInterface>> {
        let sys = get_interface::<sys::VR_IVRSystem_FnTable>(sys::IVRSystem_Version)?;

        Ok(Handle(Rc::new(SystemInterface {
            sys: unsafe { CastRc::new(self.0.clone(), sys) },
        })))
    }

    pub fn compositor(&self) -> Result<Handle<CompositorInterface>> {
        let sys = get_interface::<sys::VR_IVRCompositor_FnTable>(sys::IVRCompositor_Version)?;

        Ok(Handle(Rc::new(CompositorInterface {
            sys: unsafe { CastRc::new(self.0.clone(), sys) },
        })))
    }

    pub fn input(&self, action_manifest_path: Option<PathBuf>) -> Result<input::generated::Input> {
        let sys = get_interface::<sys::VR_IVRInput_FnTable>(sys::IVRInput_Version)?;

        if let Some(path) = &action_manifest_path {
            log::info!("Setting action manifest path: {}", path.display());

            let path = path.canonicalize().unwrap();

            // TODO: Multibyte string conversion?
            let error = unsafe {
                sys.SetActionManifestPath.unwrap()(
                    std::ffi::CString::new(path.to_string_lossy().as_bytes())?
                        .as_ptr()
                        .cast_mut(),
                )
            };

            if error != sys::EVRInputError_VRInputError_None {
                return Err(anyhow::anyhow!(
                    "Failed to set action manifest path: {}",
                    error
                ));
            }
        }

        let input = input::generated::Input::new(unsafe { CastRc::new(self.0.clone(), sys) })?;

        Ok(input)
    }
}

impl Drop for OpenVr {
    fn drop(&mut self) {
        debug!("Dropping OpenVR");
        unsafe {
            sys::VR_ShutdownInternal();
        }
    }
}

pub struct SystemInterface {
    sys: CastRc<sys::VR_IVRSystem_FnTable>,
}

impl SystemInterface {}

#[derive(Debug, Clone)]
pub struct CompositorInterface {
    sys: CastRc<sys::VR_IVRCompositor_FnTable>,
}

impl Handle<CompositorInterface> {
    pub fn get_vulkan_instance_extensions_required(&self) -> Result<Vec<String>> {
        let mut extensions: [u8; 4096] = [0; 4096];

        unsafe {
            self.0
                .sys
                .get()
                .GetVulkanInstanceExtensionsRequired
                .unwrap()(
                extensions.as_mut_ptr().cast::<i8>(),
                4096,
                /*extensions.len() as u32*/
            )
        };

        let result = CStr::from_bytes_until_nul(&extensions)?
            .to_string_lossy()
            .split_ascii_whitespace()
            .map(std::string::ToString::to_string)
            .collect();

        debug!("Vulkan instance extensions: {result:?}");

        Ok(result)
    }

    pub fn get_vulkan_device_extensions_required(
        &self,
        device: &vulkano::device::physical::PhysicalDevice,
    ) -> Result<Vec<String>> {
        let mut extensions: [u8; 4096] = [0; 4096];

        unsafe {
            self.0.sys.get().GetVulkanDeviceExtensionsRequired.unwrap()(
                device.handle().as_raw() as *mut sys::VkPhysicalDevice_T,
                extensions.as_mut_ptr().cast::<i8>(),
                4096,
            )
        };

        let result = CStr::from_bytes_until_nul(&extensions)?
            .to_string_lossy()
            .split_ascii_whitespace()
            .map(std::string::ToString::to_string)
            .collect();

        debug!("Vulkan device extensions: {result:?}");

        Ok(result)
    }
}

#[derive(Clone)]
pub struct OverlayInterface {
    sys: CastRc<sys::VR_IVROverlay_FnTable>,
}

impl Handle<OverlayInterface> {
    pub fn create(&self, overlay_key: &str, overlay_name: &str) -> Result<Overlay> {
        let Ok(overlay_key) = std::ffi::CString::new(overlay_key) else {
            return Err(anyhow!("Failed to create overlay key"));
        };
        let Ok(overlay_name) = std::ffi::CString::new(overlay_name) else {
            return Err(anyhow!("Failed to create overlay name"));
        };

        let mut overlay_handle: sys::VROverlayHandle_t = 0;

        let error = unsafe {
            self.0.sys.get().CreateOverlay.unwrap()(
                overlay_key.as_ptr().cast_mut(),
                overlay_name.as_ptr().cast_mut(),
                &mut overlay_handle,
            )
        };

        if error != sys::EVROverlayError_VROverlayError_None {
            return Err(anyhow::anyhow!("Failed to create overlay: {}", error));
        }

        Ok(Overlay {
            interface: self.clone(),
            overlay_handle,
        })
    }
}

pub struct Overlay {
    interface: Handle<OverlayInterface>,
    overlay_handle: sys::VROverlayHandle_t,
}

impl Overlay {
    pub fn show(&self) -> Result<()> {
        let error = unsafe { self.interface.0.sys.get().ShowOverlay.unwrap()(self.overlay_handle) };

        if error != sys::EVROverlayError_VROverlayError_None {
            return Err(anyhow::anyhow!("Failed to show overlay: {}", error));
        }

        Ok(())
    }

    pub fn hide(&self) -> Result<()> {
        let error = unsafe { self.interface.0.sys.get().HideOverlay.unwrap()(self.overlay_handle) };

        if error != sys::EVROverlayError_VROverlayError_None {
            return Err(anyhow::anyhow!("Failed to hide overlay: {}", error));
        }

        Ok(())
    }

    pub fn set_overlay_raw(
        &self,
        buffer: &[u8],
        width: u32,
        height: u32,
        bytes_per_pixel: u32,
    ) -> Result<()> {
        let error = unsafe {
            self.interface.0.sys.get().SetOverlayRaw.unwrap()(
                self.overlay_handle,
                buffer.as_ptr() as *mut c_void,
                width,
                height,
                bytes_per_pixel,
            )
        };

        trace!(
            "SetOverlayRaw: {}, {} {} {} {}",
            self.overlay_handle,
            width,
            height,
            bytes_per_pixel,
            error
        );

        if error != sys::EVROverlayError_VROverlayError_None {
            return Err(anyhow::anyhow!("Failed to set overlay raw: {}", error));
        }

        Ok(())
    }

    pub fn set_overlay_texture(&self, texture: &mut Texture) -> Result<()> {
        let TextureHandle::Vulkan(ref vulkan_image, ref queue) = texture.handle else {
            return Err(anyhow::anyhow!("Unsupported texture type"));
        };

        let mut texture_pointer = sys::VRVulkanTextureData_t {
            m_nImage: vulkan_image.handle().as_raw(),
            m_pDevice: vulkan_image.device().handle().as_raw() as *mut VkDevice_T,
            m_pPhysicalDevice: vulkan_image.device().physical_device().handle().as_raw()
                as *mut VkPhysicalDevice_T,
            m_pInstance: vulkan_image.device().instance().handle().as_raw() as *mut VkInstance_T,
            m_pQueue: queue.handle().as_raw() as *mut VkQueue_T,
            m_nQueueFamilyIndex: queue.queue_family_index(),
            m_nWidth: vulkan_image.extent()[0],
            m_nHeight: vulkan_image.extent()[1],
            m_nFormat: vulkan_image.format() as u32,
            m_nSampleCount: vulkan_image.samples() as u32,
        };

        trace!("{texture_pointer:?}");

        let mut texture = sys::Texture_t {
            handle: std::ptr::from_mut(&mut texture_pointer).cast::<std::os::raw::c_void>(),
            eType: texture.texture_type as i32,
            eColorSpace: texture.color_space as i32,
        };

        let error = unsafe {
            self.interface.0.sys.get().SetOverlayTexture.unwrap()(
                self.overlay_handle,
                std::ptr::from_mut::<sys::Texture_t>(&mut texture),
            )
        };

        if error != sys::EVROverlayError_VROverlayError_None {
            return Err(anyhow::anyhow!("Failed to set overlay texture: {}", error));
        }

        Ok(())
    }

    pub fn set_overlay_transform_absolute(
        &self,
        tracking_universe_origin: TrackingUniverseOrigin,
        transform: Affine3A,
    ) -> Result<()> {
        let error = unsafe {
            self.interface
                .0
                .sys
                .get()
                .SetOverlayTransformAbsolute
                .unwrap()(
                self.overlay_handle,
                tracking_universe_origin as sys::ETrackingUniverseOrigin,
                &mut to_hmd_matrix34_t(transform),
            )
        };

        if error != sys::EVROverlayError_VROverlayError_None {
            return Err(anyhow::anyhow!(
                "Failed to set overlay transform absolute: {}",
                error
            ));
        }

        Ok(())
    }

    pub fn wait_frame_sync(&self, timeout: u32) -> Result<()> {
        let error = unsafe { self.interface.0.sys.get().WaitFrameSync.unwrap()(timeout) };

        if error != sys::EVROverlayError_VROverlayError_None {
            return Err(anyhow::anyhow!(
                "Failed to wait for overlay frame: {}",
                error
            ));
        }

        Ok(())
    }
}

impl Drop for Overlay {
    fn drop(&mut self) {
        debug!("Dropping Overlay");
        unsafe {
            self.interface.0.sys.get().DestroyOverlay.unwrap()(self.overlay_handle);
        }
    }
}

pub fn from_hmd_matrix34_t(matrix: sys::HmdMatrix34_t) -> Affine3A {
    Affine3A::from_cols_array(&[
        matrix.m[0][0],
        matrix.m[1][0],
        matrix.m[2][0],
        matrix.m[0][1],
        matrix.m[1][1],
        matrix.m[2][1],
        matrix.m[0][2],
        matrix.m[1][2],
        matrix.m[2][2],
        matrix.m[0][3],
        matrix.m[1][3],
        matrix.m[2][3],
    ])
}

pub fn to_hmd_matrix34_t(matrix: Affine3A) -> sys::HmdMatrix34_t {
    sys::HmdMatrix34_t {
        m: [
            [
                matrix.x_axis.x,
                matrix.y_axis.x,
                matrix.z_axis.x,
                matrix.translation.x,
            ],
            [
                matrix.x_axis.y,
                matrix.y_axis.y,
                matrix.z_axis.y,
                matrix.translation.y,
            ],
            [
                matrix.x_axis.z,
                matrix.y_axis.z,
                matrix.z_axis.z,
                matrix.translation.z,
            ],
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Affine3A;

    #[test]
    fn test_hmdmatrix_conversions() {
        let a = Affine3A::from_cols_array(&[
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
        ]);

        let b = to_hmd_matrix34_t(a);

        let c = from_hmd_matrix34_t(b);

        assert!((a.translation - c.translation).length() < 1e-6);
        assert!((a.x_axis - c.x_axis).length() < 1e-6);
        assert!((a.y_axis - c.y_axis).length() < 1e-6);
        assert!((a.z_axis - c.z_axis).length() < 1e-6);
    }
}
