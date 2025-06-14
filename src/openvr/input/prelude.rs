use std::path::PathBuf;

use crate::openvr::{from_hmd_matrix34_t, CastRc, Handle, OpenVr, TrackingUniverseOrigin};
pub use crate::prelude::*;
use glam::Vec3;
use openvr_sys::{
    self as sys, ETrackingUniverseOrigin_TrackingUniverseRawAndUncalibrated, VRActiveActionSet_t,
};

#[derive(Debug, Clone)]
pub struct BooleanInput {
    pub active: bool,
    pub state: bool,
    pub changed: bool,
    pub update_time: f32,
}

#[derive(Debug, Clone)]
pub struct Vector1Input {
    pub active: bool,
    pub value: f32,
    pub delta_value: f32,
    pub update_time: f32,
}

#[derive(Debug, Clone)]
pub struct Vector2Input {
    pub active: bool,
    pub value: Vec2,
    pub delta: Vec2,
    pub update_time: f32,
}

#[derive(Debug, Clone)]
pub struct Vector3Input {
    pub active: bool,
    pub value: Vec3A,
    pub delta: Vec3A,
    pub update_time: f32,
}

#[derive(Debug, Clone)]
pub struct PoseInput {
    pub active: bool,
    pub pose: Option<Affine3A>,
}

pub struct Input {
    sys: CastRc<sys::VR_IVRInput_FnTable>,
    active_action_sets: Vec<sys::VRActiveActionSet_t>,
    generated_fields: GeneratedFields,
}

impl Input {
    pub(in crate::openvr) fn new(sys: CastRc<sys::VR_IVRInput_FnTable>) -> Result<Self> {
        let input = Input {
            active_action_sets: vec![],
            generated_fields: Self::generate_fields(sys.get())?,
            sys,
        };

        Ok(input)
    }

    pub fn update(&mut self) -> Result<()> {
        let len = u32::try_from(self.active_action_sets.len())?;

        let result = unsafe {
            self.sys.get().UpdateActionState.unwrap()(
                self.active_action_sets.as_mut_ptr(),
                u32::try_from(std::mem::size_of::<VRActiveActionSet_t>())?,
                len,
            )
        };

        if result != sys::EVRInputError_VRInputError_None {
            return Err(anyhow::anyhow!(
                "Failed to update action state: {:?}",
                result
            ));
        }

        Ok(())
    }

    pub fn get_action_handle(
        sys: &sys::VR_IVRInput_FnTable,
        action_name: &str,
    ) -> Result<sys::VRActionHandle_t> {
        let mut action_handle = sys::VRActionHandle_t::default();
        let c_action_name = std::ffi::CString::new(action_name).unwrap();

        let result = unsafe {
            sys.GetActionHandle.unwrap()(c_action_name.as_ptr().cast_mut(), &mut action_handle)
        };

        if result != sys::EVRInputError_VRInputError_None {
            return Err(anyhow::anyhow!(
                "Failed to get action handle for '{}': {:?}",
                action_name,
                result
            ));
        }

        Ok(action_handle)
    }

    pub fn get_action_set_handle(
        sys: &sys::VR_IVRInput_FnTable,
        action_set_name: &str,
    ) -> Result<sys::VRActionSetHandle_t> {
        let mut action_set_handle = sys::VRActionSetHandle_t::default();
        let c_action_set_name = std::ffi::CString::new(action_set_name).unwrap();

        let result = unsafe {
            sys.GetActionSetHandle.unwrap()(
                c_action_set_name.as_ptr().cast_mut(),
                &mut action_set_handle,
            )
        };

        if result != sys::EVRInputError_VRInputError_None {
            return Err(anyhow::anyhow!(
                "Failed to get action set handle for '{}': {:?}",
                action_set_name,
                result
            ));
        }

        Ok(action_set_handle)
    }

    fn activate_action_set(&mut self, action_set_handle: sys::VRActionSetHandle_t) {
        let active_action_set = sys::VRActiveActionSet_t {
            ulActionSet: action_set_handle,
            ulRestrictedToDevice: 0,
            ulSecondaryActionSet: 0,
            unPadding: 0,
            nPriority: 0,
        };

        self.deactivate_action_set(action_set_handle);

        self.active_action_sets.push(active_action_set);
    }

    fn deactivate_action_set(&mut self, action_set_handle: sys::VRActionSetHandle_t) {
        self.active_action_sets
            .retain(|set| set.ulActionSet != action_set_handle);
    }

    fn get_digital_action_data(
        &self,
        action_handle: sys::VRActionHandle_t,
    ) -> Result<BooleanInput> {
        let mut data = sys::InputDigitalActionData_t {
            bActive: false,
            bState: false,
            bChanged: false,
            fUpdateTime: 0.0,
            activeOrigin: 0,
        };

        log::trace!("Getting digital action data for handle: {action_handle:?}");

        let result = unsafe {
            self.sys.get().GetDigitalActionData.unwrap()(
                action_handle,
                &mut data,
                u32::try_from(std::mem::size_of::<sys::InputDigitalActionData_t>())?,
                sys::k_ulInvalidInputValueHandle,
            )
        };

        if result != sys::EVRInputError_VRInputError_None {
            return Err(anyhow::anyhow!(
                "Failed to get digital action data: {:?}",
                result
            ));
        }

        Ok(BooleanInput {
            active: data.bActive,
            state: data.bState,
            changed: data.bChanged,
            update_time: data.fUpdateTime,
        })
    }

    fn get_analog_action_data(
        &self,
        action_handle: sys::VRActionHandle_t,
    ) -> Result<sys::InputAnalogActionData_t> {
        let mut data = sys::InputAnalogActionData_t {
            bActive: false,
            fUpdateTime: 0.0,
            activeOrigin: 0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            deltaX: 0.0,
            deltaY: 0.0,
            deltaZ: 0.0,
        };

        log::trace!("Getting analog action data for handle: {action_handle:?}");

        let result = unsafe {
            self.sys.get().GetAnalogActionData.unwrap()(
                action_handle,
                &mut data,
                u32::try_from(std::mem::size_of::<sys::InputAnalogActionData_t>())?,
                sys::k_ulInvalidInputValueHandle,
            )
        };

        if result != sys::EVRInputError_VRInputError_None {
            return Err(anyhow::anyhow!(
                "Failed to get analog action data: {:?}",
                result
            ));
        }

        Ok(data)
    }

    fn get_vector1_action_data(
        &self,
        action_handle: sys::VRActionHandle_t,
    ) -> Result<Vector1Input> {
        let data = self.get_analog_action_data(action_handle)?;

        Ok(Vector1Input {
            active: data.bActive,
            value: data.x,
            delta_value: data.deltaX,
            update_time: data.fUpdateTime,
        })
    }

    fn get_vector2_action_data(
        &self,
        action_handle: sys::VRActionHandle_t,
    ) -> Result<Vector2Input> {
        let data = self.get_analog_action_data(action_handle)?;

        Ok(Vector2Input {
            active: data.bActive,
            value: Vec2::new(data.x, data.y),
            delta: Vec2::new(data.deltaX, data.deltaY),
            update_time: data.fUpdateTime,
        })
    }

    fn get_vector3_action_data(
        &self,
        action_handle: sys::VRActionHandle_t,
    ) -> Result<Vector3Input> {
        let data = self.get_analog_action_data(action_handle)?;

        Ok(Vector3Input {
            active: data.bActive,
            value: Vec3A::new(data.x, data.y, data.z),
            delta: Vec3A::new(data.deltaX, data.deltaY, data.deltaZ),
            update_time: data.fUpdateTime,
        })
    }

    fn get_pose_action_data(
        &self,
        tracking_universe_origin: TrackingUniverseOrigin,
        action_handle: sys::VRActionHandle_t,
    ) -> Result<PoseInput> {
        let mut data = sys::InputPoseActionData_t {
            bActive: false,
            activeOrigin: 0,
            pose: sys::TrackedDevicePose_t {
                mDeviceToAbsoluteTracking: sys::HmdMatrix34_t {
                    m: [
                        [0.0, 0.0, 0.0, 0.0],
                        [0.0, 0.0, 0.0, 0.0],
                        [0.0, 0.0, 0.0, 0.0],
                    ],
                },
                vVelocity: sys::HmdVector3_t { v: [0.0, 0.0, 0.0] },
                vAngularVelocity: sys::HmdVector3_t { v: [0.0, 0.0, 0.0] },
                eTrackingResult: 0,
                bPoseIsValid: false,
                bDeviceIsConnected: false,
            },
        };

        let result = unsafe {
            self.sys.get().GetPoseActionDataForNextFrame.unwrap()(
                action_handle,
                tracking_universe_origin as sys::ETrackingUniverseOrigin, // TODO: Origin
                &mut data,
                u32::try_from(std::mem::size_of::<sys::InputPoseActionData_t>())?,
                sys::k_ulInvalidInputValueHandle,
            )
        };

        if result != sys::EVRInputError_VRInputError_None {
            return Err(anyhow::anyhow!(
                "Failed to get pose action data: {:?}",
                result
            ));
        }

        Ok(PoseInput {
            active: data.bActive,
            pose: Some(from_hmd_matrix34_t(data.pose.mDeviceToAbsoluteTracking)),
        })
    }
}

// STUB_FOLLOWS

// NOTE: The following codes are stubs and removed when the code is generated.

impl Input {
    fn set_action_manifest_path() {
        todo!();
    }

    fn generate_fields(_sys: &sys::VR_IVRInput_FnTable) -> Result<GeneratedFields> {
        todo!();
    }
}

pub struct GeneratedFields {
    some_action: sys::VRActionHandle_t,
    some_action_set: sys::VRActionSetHandle_t,
}
