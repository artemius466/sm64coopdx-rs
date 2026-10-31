//! 3D vectors and 4x4 matrices.
//!
//! Mirrors `include/types.h`:
//!   - `Vec3f` is `f32[3]` with Y as up.
//!   - `Vec3s` is `s16[3]`.
//!   - `Mat4` is `f32[4][4]` (column-major, matching the RDP expectation).
//!
//! The game stores transforms as `Mat4` and positions as `Vec3f`/`Vec3s`.

use crate::types::{f32, s16};
use std::ops::{Add, Sub};

/// A 3D vector of floats. Y is up, matching the game's convention.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Vec3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// A 3D vector of 16-bit ints.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Vec3s {
    pub x: s16,
    pub y: s16,
    pub z: s16,
}

/// A 3D vector of 32-bit ints.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Vec3i {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// A column-major 4x4 matrix (matches the RDP's expectation).
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Mat4 {
    pub entries: [[f32; 4]; 4],
}

impl Vec3f {
    #[inline]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub fn zero() -> Self {
        Self::default()
    }

    /// Add two vectors.
    #[inline]
    pub fn add(&self, other: &Vec3f) -> Vec3f {
        Vec3f::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }

    /// Subtract another vector.
    #[inline]
    pub fn sub(&self, other: &Vec3f) -> Vec3f {
        Vec3f::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }

    /// Scale by a scalar.
    #[inline]
    pub fn scale(&self, factor: f32) -> Vec3f {
        Vec3f::new(self.x * factor, self.y * factor, self.z * factor)
    }

    /// Dot product.
    #[inline]
    pub fn dot(&self, other: &Vec3f) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Cross product.
    #[inline]
    pub fn cross(&self, other: &Vec3f) -> Vec3f {
        Vec3f::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    /// Squared length (no sqrt — cheaper for comparisons).
    #[inline]
    pub fn length_squared(&self) -> f32 {
        self.dot(self)
    }

    /// Euclidean length.
    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Normalised copy (returns zero vector for a zero-length vector).
    pub fn normalized(&self) -> Vec3f {
        let len = self.length();
        if len == 0.0 {
            *self
        } else {
            Vec3f::new(self.x / len, self.y / len, self.z / len)
        }
    }

    /// Distance to another vector.
    #[inline]
    pub fn distance_to(&self, other: &Vec3f) -> f32 {
        self.sub(other).length()
    }
}

impl Add for Vec3f {
    type Output = Vec3f;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Vec3f::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3f {
    type Output = Vec3f;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Vec3f::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mat4 {
    /// The identity matrix.
    #[inline]
    pub fn identity() -> Self {
        let mut m = Self::default();
        let e = &mut m.entries;
        e[0][0] = 1.0;
        e[1][1] = 1.0;
        e[2][2] = 1.0;
        e[3][3] = 1.0;
        m
    }

    /// Multiply two matrices (this * other), column-major.
    #[inline]
    pub fn multiply(&self, other: &Mat4) -> Mat4 {
        let mut out = Mat4::identity();
        let a = &self.entries;
        let b = &other.entries;
        let o = &mut out.entries;
        for col in 0..4 {
            for row in 0..4 {
                o[row][col] = (0..4)
                    .map(|k| a[row][k] * b[k][col])
                    .fold(0.0, |acc, v| acc + v);
            }
        }
        out
    }

    /// Transform a point by this matrix.
    pub fn transform_point(&self, p: &Vec3f) -> Vec3f {
        let e = &self.entries;
        Vec3f::new(
            e[0][0] * p.x + e[1][0] * p.y + e[2][0] * p.z + e[3][0],
            e[0][1] * p.x + e[1][1] * p.y + e[2][1] * p.z + e[3][1],
            e[0][2] * p.x + e[1][2] * p.y + e[2][2] * p.z + e[3][2],
        )
    }

    /// Build a translation matrix.
    pub fn translation(x: f32, y: f32, z: f32) -> Mat4 {
        let mut m = Mat4::identity();
        m.entries[3][0] = x;
        m.entries[3][1] = y;
        m.entries[3][2] = z;
        m
    }

    /// Build a uniform/non-uniform scale matrix.
    pub fn scale(x: f32, y: f32, z: f32) -> Mat4 {
        let mut m = Mat4::identity();
        m.entries[0][0] = x;
        m.entries[1][1] = y;
        m.entries[2][2] = z;
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec3_add_sub_scale() {
        let a = Vec3f::new(1.0, 2.0, 3.0);
        let b = Vec3f::new(4.0, 5.0, 6.0);
        assert!(((a + b) - Vec3f::new(5.0, 7.0, 9.0)).length() < 1e-6);
        assert!(((a - b) - Vec3f::new(-3.0, -3.0, -3.0)).length() < 1e-6);
        assert!((a.scale(2.0) - Vec3f::new(2.0, 4.0, 6.0)).length() < 1e-6);
    }

    #[test]
    fn vec3_dot_cross() {
        let a = Vec3f::new(1.0, 0.0, 0.0);
        let b = Vec3f::new(0.0, 1.0, 0.0);
        assert!((a.dot(&b)).abs() < 1e-6);
        let c = a.cross(&b);
        assert!((c - Vec3f::new(0.0, 0.0, 1.0)).length() < 1e-6);
    }

    #[test]
    fn mat4_identity_transform() {
        let m = Mat4::identity();
        let p = Vec3f::new(3.0, 4.0, 5.0);
        assert!((m.transform_point(&p) - p).length() < 1e-6);
    }

    #[test]
    fn mat4_multiply_associativity() {
        let a = Mat4::translation(1.0, 2.0, 3.0);
        let b = Mat4::scale(2.0, 2.0, 2.0);
        let c = Mat4::translation(0.0, 0.0, 1.0);
        // (a*b)*c == a*(b*c)
        let lhs = a.multiply(&b).multiply(&c);
        let rhs = a.multiply(&b.multiply(&c));
        assert!((lhs.entries[3][0] - rhs.entries[3][0]).abs() < 1e-5);
        assert!((lhs.entries[3][1] - rhs.entries[3][1]).abs() < 1e-5);
        assert!((lhs.entries[3][2] - rhs.entries[3][2]).abs() < 1e-5);
    }
}
