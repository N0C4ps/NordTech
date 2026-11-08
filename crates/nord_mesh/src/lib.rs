//! Backend-independent indexed triangle mesh data and validation.
//!
//! Meshes own CPU-side vertices, indices and bounds only. GPU buffers,
//! materials and draw scheduling belong to renderer implementations.

use std::error::Error;
use std::fmt::{Display, Formatter};

/// Basic vertex attributes shared by the initial static-mesh pipeline.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshVertex {
    position: [f32; 3],
    normal: [f32; 3],
}

impl MeshVertex {
    /// Create a vertex with a finite position and a normalized surface normal.
    ///
    /// # Errors
    /// Returns [`MeshError`] for non-finite data or a zero-length normal.
    pub fn new(position: [f32; 3], normal: [f32; 3]) -> Result<Self, MeshError> {
        if !position.iter().chain(normal.iter()).all(|component| component.is_finite()) {
            return Err(MeshError::NonFiniteVertex);
        }
        let length = normal.iter().map(|component| component * component).sum::<f32>().sqrt();
        if !length.is_finite() || length <= f32::EPSILON {
            return Err(MeshError::DegenerateNormal);
        }
        let normal = normal.map(|component| component / length);
        Ok(Self { position, normal })
    }

    /// Local-space position.
    #[must_use]
    pub const fn position(self) -> [f32; 3] {
        self.position
    }

    /// Unit local-space normal.
    #[must_use]
    pub const fn normal(self) -> [f32; 3] {
        self.normal
    }
}

/// Validated CPU-side indexed triangle mesh.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexedMesh {
    vertices: Vec<MeshVertex>,
    indices: Vec<u32>,
    bounds: MeshBounds,
}

impl IndexedMesh {
    /// Construct a triangle-list mesh and calculate its local-space bounds.
    ///
    /// # Errors
    /// Returns an error when vertices/indices are empty, index count is not a
    /// triangle list, an index is out of range, or bounds cannot be formed.
    pub fn new(vertices: Vec<MeshVertex>, indices: Vec<u32>) -> Result<Self, MeshError> {
        if vertices.is_empty() {
            return Err(MeshError::EmptyVertices);
        }
        if indices.is_empty() {
            return Err(MeshError::EmptyIndices);
        }
        if !indices.len().is_multiple_of(3) {
            return Err(MeshError::IndexCountNotTriangleList);
        }
        let vertex_count = u32::try_from(vertices.len()).map_err(|_| MeshError::TooManyVertices)?;
        if indices.iter().any(|&index| index >= vertex_count) {
            return Err(MeshError::IndexOutOfRange);
        }
        let mut minimum = [f32::INFINITY; 3];
        let mut maximum = [f32::NEG_INFINITY; 3];
        for vertex in &vertices {
            for axis in 0..3 {
                minimum[axis] = minimum[axis].min(vertex.position[axis]);
                maximum[axis] = maximum[axis].max(vertex.position[axis]);
            }
        }
        let bounds = MeshBounds { min: minimum, max: maximum };
        Ok(Self { vertices, indices, bounds })
    }

    /// Vertices in the mesh's local coordinate space.
    #[must_use]
    pub fn vertices(&self) -> &[MeshVertex] {
        &self.vertices
    }

    /// Triangle-list indices into [`Self::vertices`].
    #[must_use]
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// Axis-aligned local-space bounds derived from all vertices.
    #[must_use]
    pub const fn bounds(&self) -> MeshBounds {
        self.bounds
    }
}

/// Axis-aligned bounds in the mesh's local space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshBounds {
    min: [f32; 3],
    max: [f32; 3],
}

impl MeshBounds {
    /// Minimum corner.
    #[must_use]
    pub const fn min(self) -> [f32; 3] {
        self.min
    }

    /// Maximum corner.
    #[must_use]
    pub const fn max(self) -> [f32; 3] {
        self.max
    }
}

/// Invalid mesh data rejected before GPU upload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MeshError {
    EmptyVertices,
    EmptyIndices,
    IndexCountNotTriangleList,
    IndexOutOfRange,
    TooManyVertices,
    NonFiniteVertex,
    DegenerateNormal,
}

impl Display for MeshError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "invalid indexed mesh: {self:?}")
    }
}

impl Error for MeshError {}

#[cfg(test)]
mod tests {
    use super::{IndexedMesh, MeshError, MeshVertex};
    #[test]
    fn validates_indexed_triangle_mesh_and_derives_bounds() {
        let vertices = vec![
            MeshVertex::new([-1.0, -2.0, -3.0], [0.0, 1.0, 0.0]).unwrap(),
            MeshVertex::new([1.0, 2.0, 3.0], [0.0, 1.0, 0.0]).unwrap(),
            MeshVertex::new([0.0; 3], [0.0, 1.0, 0.0]).unwrap(),
        ];
        let mesh = IndexedMesh::new(vertices, vec![0, 1, 2]).unwrap();
        assert_eq!(mesh.indices(), &[0, 1, 2]);
        assert!(
            mesh.bounds()
                .min()
                .iter()
                .zip([-1.0, -2.0, -3.0])
                .all(|(actual, expected)| { (actual - expected).abs() < f32::EPSILON })
        );
        assert!(
            mesh.bounds()
                .max()
                .iter()
                .zip([1.0, 2.0, 3.0])
                .all(|(actual, expected)| { (actual - expected).abs() < f32::EPSILON })
        );
    }

    #[test]
    fn rejects_malformed_triangle_indices_and_normals() {
        let vertex = MeshVertex::new([0.0; 3], [0.0, 1.0, 0.0]).unwrap();
        assert_eq!(
            IndexedMesh::new(vec![vertex; 3], vec![0, 1]),
            Err(MeshError::IndexCountNotTriangleList)
        );
        assert_eq!(
            IndexedMesh::new(vec![vertex; 3], vec![0, 1, 3]),
            Err(MeshError::IndexOutOfRange)
        );
        assert_eq!(MeshVertex::new([0.0; 3], [0.0; 3]), Err(MeshError::DegenerateNormal));
    }
}
