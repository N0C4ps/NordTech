//! Backend-independent perspective camera state and projection math.
//!
//! The API uses plain arrays so it can be used without depending on a shared
//! engine math crate. Consumers can convert matrices and coordinates at their
//! application boundary.

use std::error::Error;
use std::fmt::{Display, Formatter};

/// Row-major 4x4 matrix acting on column vectors.
pub type CameraMatrix4 = [[f32; 4]; 4];
type Vec3 = [f32; 3];

/// Invalid camera lens or pose parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CameraError {
    NonFiniteParameter,
    InvalidFieldOfView,
    InvalidAspectRatio,
    InvalidClipPlanes,
    UnrepresentableProjection,
    DegenerateDirection,
    DegenerateUp,
    ParallelDirectionAndUp,
}

impl Display for CameraError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::NonFiniteParameter => "camera parameters must be finite",
            Self::InvalidFieldOfView => "vertical field of view must be between zero and pi",
            Self::InvalidAspectRatio => "camera aspect ratio must be greater than zero",
            Self::InvalidClipPlanes => "camera clip planes require 0 < near < far",
            Self::UnrepresentableProjection => {
                "camera projection cannot be represented with finite matrix values"
            }
            Self::DegenerateDirection => "camera forward direction must be non-zero",
            Self::DegenerateUp => "camera up direction must be non-zero",
            Self::ParallelDirectionAndUp => "camera forward and up directions must not be parallel",
        })
    }
}

impl Error for CameraError {}

/// Right-handed perspective camera using a zero-to-one depth range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerspectiveCamera {
    position: Vec3,
    forward: Vec3,
    up: Vec3,
    vertical_fov_radians: f32,
    aspect_ratio: f32,
    near_plane: f32,
    far_plane: f32,
}

impl PerspectiveCamera {
    /// Create a perspective camera at the origin looking down negative Z.
    ///
    /// # Errors
    ///
    /// Returns an error when the lens parameters are invalid or produce a
    /// projection matrix that cannot be represented with finite `f32` values.
    pub fn new(
        vertical_fov_radians: f32,
        aspect_ratio: f32,
        near_plane: f32,
        far_plane: f32,
    ) -> Result<Self, CameraError> {
        validate_lens(vertical_fov_radians, aspect_ratio, near_plane, far_plane)?;
        if projection(vertical_fov_radians, aspect_ratio, near_plane, far_plane).is_none() {
            return Err(CameraError::UnrepresentableProjection);
        }
        Ok(Self {
            position: [0.0; 3],
            forward: [0.0, 0.0, -1.0],
            up: [0.0, 1.0, 0.0],
            vertical_fov_radians,
            aspect_ratio,
            near_plane,
            far_plane,
        })
    }

    /// Update the camera pose after validating that the basis is usable.
    ///
    /// # Errors
    ///
    /// Returns an error when the pose contains non-finite values or the
    /// forward and up vectors cannot form a valid camera basis.
    pub fn set_pose(
        &mut self,
        position: [f32; 3],
        forward: [f32; 3],
        up: [f32; 3],
    ) -> Result<(), CameraError> {
        if !finite3(position) || !finite3(forward) || !finite3(up) {
            return Err(CameraError::NonFiniteParameter);
        }
        let forward = normalize(forward).ok_or(CameraError::DegenerateDirection)?;
        let up = normalize(up).ok_or(CameraError::DegenerateUp)?;
        if normalize(cross(forward, up)).is_none() {
            return Err(CameraError::ParallelDirectionAndUp);
        }
        if view(position, forward, up).is_none() {
            return Err(CameraError::NonFiniteParameter);
        }
        self.position = position;
        self.forward = forward;
        self.up = up;
        Ok(())
    }

    /// Point the camera from `position` toward `target`.
    ///
    /// # Errors
    ///
    /// Returns an error when the target or pose is non-finite, the direction
    /// is degenerate, or the up vector cannot form a valid camera basis.
    pub fn look_at(
        &mut self,
        position: [f32; 3],
        target: [f32; 3],
        up: [f32; 3],
    ) -> Result<(), CameraError> {
        if !finite3(target) {
            return Err(CameraError::NonFiniteParameter);
        }
        self.set_pose(position, sub(target, position), up)
    }

    /// Change the aspect ratio, typically after a non-zero surface resize.
    ///
    /// # Errors
    ///
    /// Returns an error when the ratio is non-finite, non-positive, or would
    /// produce a projection matrix with non-finite values.
    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) -> Result<(), CameraError> {
        if !aspect_ratio.is_finite() {
            return Err(CameraError::NonFiniteParameter);
        }
        if aspect_ratio <= 0.0 {
            return Err(CameraError::InvalidAspectRatio);
        }
        if projection(self.vertical_fov_radians, aspect_ratio, self.near_plane, self.far_plane)
            .is_none()
        {
            return Err(CameraError::UnrepresentableProjection);
        }
        self.aspect_ratio = aspect_ratio;
        Ok(())
    }

    /// World-space camera position.
    #[must_use]
    pub const fn position(self) -> [f32; 3] {
        self.position
    }

    /// Unit world-space forward direction.
    #[must_use]
    pub const fn forward(self) -> [f32; 3] {
        self.forward
    }

    /// Current viewport aspect ratio.
    #[must_use]
    pub const fn aspect_ratio(self) -> f32 {
        self.aspect_ratio
    }

    /// Build the world-to-view matrix.
    ///
    /// # Errors
    ///
    /// Returns [`CameraError::NonFiniteParameter`] if the matrix cannot be
    /// represented with finite values.
    pub fn view_matrix(self) -> Result<CameraMatrix4, CameraError> {
        view(self.position, self.forward, self.up).ok_or(CameraError::NonFiniteParameter)
    }

    /// Build a right-handed perspective matrix with `[0, 1]` clip depth.
    ///
    /// # Errors
    ///
    /// Returns [`CameraError::UnrepresentableProjection`] if the matrix cannot
    /// be represented with finite values.
    pub fn projection_matrix(self) -> Result<CameraMatrix4, CameraError> {
        projection(self.vertical_fov_radians, self.aspect_ratio, self.near_plane, self.far_plane)
            .ok_or(CameraError::UnrepresentableProjection)
    }

    /// Build the matrix that transforms world-space positions to clip space.
    ///
    /// # Errors
    ///
    /// Returns an error if either component matrix or their product contains
    /// values that cannot be represented finitely.
    pub fn view_projection_matrix(self) -> Result<CameraMatrix4, CameraError> {
        let projection = self.projection_matrix()?;
        let view = self.view_matrix()?;
        let mut result = [[0.0; 4]; 4];
        for (row, out_row) in result.iter_mut().enumerate() {
            for (column, value) in out_row.iter_mut().enumerate() {
                *value = (0..4).map(|index| projection[row][index] * view[index][column]).sum();
            }
        }
        result
            .iter()
            .flatten()
            .all(|value| value.is_finite())
            .then_some(result)
            .ok_or(CameraError::NonFiniteParameter)
    }
}

fn validate_lens(fov: f32, aspect: f32, near: f32, far: f32) -> Result<(), CameraError> {
    if !fov.is_finite() || !aspect.is_finite() || !near.is_finite() || !far.is_finite() {
        return Err(CameraError::NonFiniteParameter);
    }
    if fov <= 0.0 || fov >= std::f32::consts::PI {
        return Err(CameraError::InvalidFieldOfView);
    }
    if aspect <= 0.0 {
        return Err(CameraError::InvalidAspectRatio);
    }
    if near <= 0.0 || far <= near {
        return Err(CameraError::InvalidClipPlanes);
    }
    Ok(())
}

fn projection(fov: f32, aspect: f32, near: f32, far: f32) -> Option<CameraMatrix4> {
    let y = 1.0 / (fov * 0.5).tan();
    let x = y / aspect;
    let depth = far / (near - far);
    let offset = far * near / (near - far);
    let matrix =
        [[x, 0.0, 0.0, 0.0], [0.0, y, 0.0, 0.0], [0.0, 0.0, depth, offset], [0.0, 0.0, -1.0, 0.0]];
    matrix.iter().flatten().all(|value| value.is_finite()).then_some(matrix)
}

fn view(position: Vec3, forward: Vec3, up: Vec3) -> Option<CameraMatrix4> {
    let side = normalize(cross(forward, up))?;
    let corrected_up = cross(side, forward);
    let matrix = [
        [side[0], side[1], side[2], -dot(side, position)],
        [corrected_up[0], corrected_up[1], corrected_up[2], -dot(corrected_up, position)],
        [-forward[0], -forward[1], -forward[2], dot(forward, position)],
        [0.0, 0.0, 0.0, 1.0],
    ];
    matrix.iter().flatten().all(|value| value.is_finite()).then_some(matrix)
}

fn finite3(value: Vec3) -> bool {
    value.iter().all(|component| component.is_finite())
}
fn dot(left: Vec3, right: Vec3) -> f32 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}
fn cross(left: Vec3, right: Vec3) -> Vec3 {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}
fn sub(left: Vec3, right: Vec3) -> Vec3 {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}
fn normalize(value: Vec3) -> Option<Vec3> {
    let length = dot(value, value).sqrt();
    (length.is_finite() && length > f32::EPSILON)
        .then(|| [value[0] / length, value[1] / length, value[2] / length])
}

#[cfg(test)]
mod tests {
    use super::{CameraError, PerspectiveCamera};

    #[test]
    fn rejects_invalid_lens_and_pose_parameters() {
        assert_eq!(
            PerspectiveCamera::new(1.0, 0.0, 0.1, 100.0).unwrap_err(),
            CameraError::InvalidAspectRatio
        );
        assert_eq!(
            PerspectiveCamera::new(1.0, 1.0, 0.0, 100.0).unwrap_err(),
            CameraError::InvalidClipPlanes
        );
        let mut camera = PerspectiveCamera::new(1.0, 1.0, 0.1, 100.0).unwrap();
        assert_eq!(
            camera.set_pose([0.0; 3], [0.0, 0.0, -1.0], [0.0, 0.0, -1.0]),
            Err(CameraError::ParallelDirectionAndUp)
        );
    }

    #[test]
    fn view_projection_maps_near_and_far_to_zero_one_depth() {
        let mut camera = PerspectiveCamera::new(1.0, 16.0 / 9.0, 0.1, 100.0).unwrap();
        camera.look_at([0.0, 0.0, 5.0], [0.0; 3], [0.0, 1.0, 0.0]).unwrap();
        let matrix = camera.view_projection_matrix().unwrap();
        let transform = |z: f32| {
            let point = [0.0, 0.0, z, 1.0];
            let clip: [f32; 4] =
                std::array::from_fn(|row| (0..4).map(|col| matrix[row][col] * point[col]).sum());
            clip[2] / clip[3]
        };
        assert!(transform(4.9).abs() < 1.0e-5);
        assert!((transform(-95.0) - 1.0).abs() < 1.0e-5);
    }

    #[test]
    fn resize_updates_aspect_without_changing_pose_or_lens() {
        let mut camera = PerspectiveCamera::new(1.0, 1.0, 0.1, 100.0).unwrap();
        camera.set_aspect_ratio(2.0).unwrap();
        assert!((camera.aspect_ratio() - 2.0).abs() < f32::EPSILON);
        assert!(camera.position().iter().all(|component| component.abs() < f32::EPSILON));
        assert!(
            camera
                .forward()
                .iter()
                .zip([0.0, 0.0, -1.0])
                .all(|(actual, expected)| { (actual - expected).abs() < f32::EPSILON })
        );
    }
}
