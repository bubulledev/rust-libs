//! Types mathématiques minimaux, sans allocation, utilisés dans les chemins
//! chauds (détection de collision, requêtes de blocs, tri spatial).
//!
//! Ils sont délibérément indépendants de toute bibliothèque externe : ce crate
//! n'a aucune dépendance, donc rien ne s'interpose entre votre code et le
//! compilateur.

use std::ops::{Add, Mul, Sub};

/// Vecteur 3D en `f64` (précision identique à celle du serveur Minecraft).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub const fn splat(value: f64) -> Self {
        Self::new(value, value, value)
    }

    pub fn length_squared(self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn length(self) -> f64 {
        self.length_squared().sqrt()
    }

    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    /// Normalise, ou renvoie [`Vec3::ZERO`] pour un vecteur nul.
    pub fn normalize_or_zero(self) -> Self {
        let len = self.length();
        if len > f64::EPSILON {
            self * (1.0 / len)
        } else {
            Self::ZERO
        }
    }

    pub fn distance_squared(self, other: Self) -> f64 {
        (self - other).length_squared()
    }

    pub fn distance(self, other: Self) -> f64 {
        self.distance_squared(other).sqrt()
    }

    pub fn lerp(self, other: Self, t: f64) -> Self {
        self + (other - self) * t
    }

    /// Bloc contenant ce point (arrondi vers le bas, comme Minecraft).
    pub fn floor(self) -> BlockPos {
        BlockPos::new(
            self.x.floor() as i32,
            self.y.floor() as i32,
            self.z.floor() as i32,
        )
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul<f64> for Vec3 {
    type Output = Vec3;
    fn mul(self, rhs: f64) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

/// Coordonnées entières d'un bloc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub const fn offset(self, dx: i32, dy: i32, dz: i32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.z + dz)
    }

    pub fn to_vec3(self) -> Vec3 {
        Vec3::new(self.x as f64, self.y as f64, self.z as f64)
    }

    pub fn center(self) -> Vec3 {
        self.to_vec3() + Vec3::splat(0.5)
    }

    pub fn distance_squared(self, other: Self) -> f64 {
        let dx = (self.x - other.x) as f64;
        let dy = (self.y - other.y) as f64;
        let dz = (self.z - other.z) as f64;
        dx * dx + dy * dy + dz * dz
    }

    /// Voisins directs (6 blocs partageant une face).
    pub fn neighbours(self) -> [BlockPos; 6] {
        [
            self.offset(1, 0, 0),
            self.offset(-1, 0, 0),
            self.offset(0, 1, 0),
            self.offset(0, -1, 0),
            self.offset(0, 0, 1),
            self.offset(0, 0, -1),
        ]
    }
}

/// Boîte englobante alignée sur les axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    /// Construit une AABB en réordonnant `min`/`max` si nécessaire.
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self {
            min: Vec3::new(min.x.min(max.x), min.y.min(max.y), min.z.min(max.z)),
            max: Vec3::new(min.x.max(max.x), min.y.max(max.y), min.z.max(max.z)),
        }
    }

    pub fn from_center(center: Vec3, half_extent: Vec3) -> Self {
        Self::new(center - half_extent, center + half_extent)
    }

    /// AABB d'un seul bloc.
    pub fn block(pos: BlockPos) -> Self {
        Self::new(pos.to_vec3(), pos.to_vec3() + Vec3::splat(1.0))
    }

    pub fn contains(&self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    pub fn expand(&self, amount: f64) -> Self {
        Self::new(self.min - Vec3::splat(amount), self.max + Vec3::splat(amount))
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn volume(&self) -> f64 {
        let size = self.max - self.min;
        size.x * size.y * size.z
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operations_de_base() {
        let a = Vec3::new(1.0, 2.0, 2.0);
        assert_eq!(a.length(), 3.0);
        assert_eq!((a + a), Vec3::new(2.0, 4.0, 4.0));
        assert_eq!((a - a), Vec3::ZERO);
        assert!((a * 2.0).length() - 6.0 < 1e-12);
    }

    #[test]
    fn normalisation_du_vecteur_nul() {
        assert_eq!(Vec3::ZERO.normalize_or_zero(), Vec3::ZERO);
        assert!((Vec3::new(3.0, 4.0, 0.0).normalize_or_zero().length() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn floor_et_voisins() {
        assert_eq!(Vec3::new(-1.2, 0.0, 2.9).floor(), BlockPos::new(-2, 0, 2));
        let pos = BlockPos::new(4, 5, 6);
        assert_eq!(pos.neighbours()[0], BlockPos::new(5, 5, 6));
        assert_eq!(pos.distance_squared(BlockPos::new(4, 5, 9)), 9.0);
    }

    #[test]
    fn aabb() {
        let a = Aabb::block(BlockPos::new(0, 0, 0));
        let b = Aabb::block(BlockPos::new(1, 1, 1));
        assert!(a.intersects(&b));
        assert!(!a.contains(Vec3::new(2.0, 0.5, 0.5)));
        assert!(a.contains(Vec3::new(0.5, 0.5, 0.5)));
        assert!(!Aabb::block(BlockPos::new(4, 4, 4)).intersects(&b));
        assert_eq!(a.volume(), 1.0);
        assert_eq!(Aabb::block(BlockPos::new(2, 2, 2)).center(), Vec3::new(2.5, 2.5, 2.5));
    }
}
