// src/world/mod.rs
//! World simulation including spatial grid and resources.

pub struct World;
pub struct WorldConfig;

pub struct GridConfig {
    pub size: (u32, u32, u32),
    pub chunk_size: u32,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            size: (100, 100, 10),
            chunk_size: 16,
        }
    }
}

pub struct Position;
pub struct Chunk;

impl World {
    pub fn new(_config: GridConfig) -> Self {
        Self
    }
}
