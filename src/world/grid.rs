// src/world/grid.rs
//! Spatial grid system for the world.

use serde::{Deserialize, Serialize};
use rand::Rng;
use crate::world::{Tile, TerrainType};

/// 2D Position in the world
/// Ordered, so that maps keyed by a place can be iterated the same way twice.
///
/// An agent's memory of the world is keyed by `Position` - what it knows is
/// where, what it was told and when it last looked - and the decision layer
/// searches those maps for a best or a nearest. Ordering by hash meant the
/// answer changed between runs of the same binary. See
/// `ExplorationKnowledge`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Calculate Manhattan distance to another position
    pub fn distance_to(&self, other: &Position) -> u32 {
        ((self.x - other.x).abs() + (self.y - other.y).abs()) as u32
    }

    /// Calculate Euclidean distance to another position
    pub fn euclidean_distance_to(&self, other: &Position) -> f32 {
        let dx = (self.x - other.x) as f32;
        let dy = (self.y - other.y) as f32;
        (dx * dx + dy * dy).sqrt()
    }

    /// Get neighboring positions (4-directional)
    pub fn neighbors(&self) -> Vec<Position> {
        vec![
            Position::new(self.x + 1, self.y),
            Position::new(self.x - 1, self.y),
            Position::new(self.x, self.y + 1),
            Position::new(self.x, self.y - 1),
        ]
    }

    /// Get all 8 neighboring positions (including diagonals)
    pub fn neighbors_8(&self) -> Vec<Position> {
        vec![
            Position::new(self.x + 1, self.y),
            Position::new(self.x - 1, self.y),
            Position::new(self.x, self.y + 1),
            Position::new(self.x, self.y - 1),
            Position::new(self.x + 1, self.y + 1),
            Position::new(self.x + 1, self.y - 1),
            Position::new(self.x - 1, self.y + 1),
            Position::new(self.x - 1, self.y - 1),
        ]
    }
}

/// 2D Grid containing tiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Vec<Tile>>,

    /// The ground somebody has left something on.
    ///
    /// Muck, and the seed in it. Two phases of the turn used to look for
    /// these by walking every tile in the world, which made a turn cost what
    /// the map *is* rather than what is happening on it. See
    /// `Soil::has_somebody_left_something_here` and ISSUES_FOUND.md #128.
    ///
    /// A superset on purpose. Anything that puts something on the ground says
    /// so, and may say so when nothing came of it; the pruning pass drops what
    /// turns out to be bare. Over-noting costs one visit and under-noting
    /// would leave litter lying for ever, so the sites err towards noting.
    /// Held as (row, column) so that walking it in order walks the map the
    /// same way the old sweeps did - top to bottom, left to right. Anything
    /// downstream that depends on the order things are visited in then sees
    /// exactly what it saw before.
    #[serde(default)]
    ground_with_something_on_it: std::collections::BTreeSet<(usize, usize)>,

    /// The ground somebody has broken, and what has happened to each piece of
    /// it since.
    ///
    /// The only soil state the map stores. Everywhere else is its terrain's
    /// natural ground, worked out when asked - see `SoilType::natural_to` - so
    /// a map costs nothing for its soil however large it is, and this holds a
    /// few hundred entries for a settlement's fields. Keyed (row, column) like
    /// the register above, so walking it walks the map in a fixed order.
    #[serde(default)]
    fields: std::collections::BTreeMap<(usize, usize), crate::world::soil::Field>,
}

impl Grid {
    /// How much ground one cell of this map stands for, in metres a side.
    ///
    /// A cell is a hundred square metres. This is the unit the rest of the
    /// model has always quietly been using: a forage radius of 25 cells is a
    /// quarter-kilometre walk and a sight radius of 8 is eighty metres, both
    /// of which are sensible for a person, and both of which are nonsense at
    /// a metre a cell. It is also the coarsest cell an ecology still reads
    /// properly on - one of these holds a stand of hazel or a good deal of
    /// grass, which is the scale the grazing and the seeding work at.
    pub const METRES_PER_CELL: f32 = 10.0;

    /// A thousand cells is ten kilometres is a hundred square kilometres.
    pub const HOW_MANY_CELLS_ACROSS_A_COUNTRY: usize = 1000;

    /// How much ground this map is, in square kilometres.
    pub fn how_much_ground(&self) -> f32 {
        let metres = Self::METRES_PER_CELL * Self::METRES_PER_CELL;
        (self.width * self.height) as f32 * metres / 1_000_000.0
    }

    /// The most of something a map of this many tiles should ever hold, given
    /// what a fifty by fifty map holds.
    ///
    /// A ceiling, not a carrying capacity: what a country will actually feed
    /// is a question for the grass on it. This is only the point past which a
    /// vector stops growing, and the one thing it must not be is a number
    /// somebody chose for a small map and then never revisited.
    pub fn at_the_very_outside(tiles: usize, on_a_small_map: usize) -> usize {
        let reference = 50 * 50;
        (on_a_small_map * tiles).div_ceil(reference).max(on_a_small_map)
    }

    pub fn new(width: usize, height: usize) -> Self {
        let tiles = vec![vec![Tile::default(); width]; height];

        Self {
            ground_with_something_on_it: std::collections::BTreeSet::new(),
            fields: std::collections::BTreeMap::new(),
            width,
            height,
            tiles,
        }
    }

    /// Generate procedural terrain with naturalistic distribution
    ///
    /// Creates a diverse landscape with:
    /// - Water bodies and associated riverbanks/beaches/wetlands
    /// - Mountain regions with surrounding hills
    /// - Plains, meadows, and forests
    /// - Desert regions in appropriate areas
    pub fn generate_terrain(&mut self) {
        let mut rng = crate::core::dice::roll();

        // First pass: generate base terrain using noise
        for y in 0..self.height {
            for x in 0..self.width {
                // Use multiple noise octaves for more natural terrain
                let base_noise = self.simple_noise(x as f32 * 0.1, y as f32 * 0.1);
                let detail_noise = self.simple_noise(x as f32 * 0.3, y as f32 * 0.3) * 0.3;
                let moisture_noise = self.simple_noise(x as f32 * 0.05 + 100.0, y as f32 * 0.05);
                let combined = (base_noise + detail_noise).clamp(0.0, 1.0);

                // Generate base terrain based on elevation (combined noise)
                let terrain_type = if combined < Self::AS_LOW_AS_THE_SEA_GETS {
                    // The deepest basins are sea. Everything above them that
                    // is still under water is a river or a lake, and fresh.
                    //
                    // Doing it by elevation rather than by carving an edge
                    // means the sea turns up where the ground actually falls
                    // away to it, and a world that has no deep basin has no
                    // coast - which is a world where salt has to be found
                    // inland or not at all.
                    TerrainType::Sea
                } else if combined < 0.15 {
                    TerrainType::Water
                } else if combined < 0.25 {
                    // Low-lying areas near water
                    if moisture_noise > 0.6 {
                        TerrainType::Wetland
                    } else {
                        TerrainType::Plains
                    }
                } else if combined < 0.45 {
                    // Mid-low elevation
                    if moisture_noise > 0.7 {
                        TerrainType::Meadow
                    } else if moisture_noise < 0.3 {
                        TerrainType::Desert
                    } else {
                        TerrainType::Plains
                    }
                } else if combined < 0.65 {
                    // Mid elevation
                    if moisture_noise > 0.5 {
                        TerrainType::Forest
                    } else {
                        TerrainType::Meadow
                    }
                } else if combined < 0.8 {
                    // Higher elevation
                    TerrainType::Hills
                } else {
                    // High elevation
                    TerrainType::Mountain
                };

                self.tiles[y][x].terrain.terrain_type = terrain_type;
            }
        }

        // Second pass: create transition zones (beaches, riverbanks)
        self.generate_transition_terrain();

        // Third pass: ensure some minimum terrain diversity
        self.ensure_terrain_diversity(&mut rng);
    }

    /// How low the ground has to fall before the water standing on it is
    /// salt.
    ///
    /// Under half of what is already water, so most of a world's water stays
    /// fresh and a coast is a feature of the map rather than the whole of it.
    const AS_LOW_AS_THE_SEA_GETS: f32 = 0.06;

    /// How often a wetland beside the sea is brackish rather than fresh.
    const HOW_OFTEN_A_MARSH_BY_THE_SEA_IS_SALT: f32 = 0.75;

    /// And how often dry ground beside the sea has dried out salt.
    const HOW_OFTEN_DRY_GROUND_BY_THE_SEA_IS_A_FLAT: f32 = 0.35;

    /// Generate transition terrain between water and land
    fn generate_transition_terrain(&mut self) {
        let mut rng = crate::core::dice::roll();

        // Create a copy of terrain types for reference
        let terrain_copy: Vec<Vec<TerrainType>> = self.tiles.iter()
            .map(|row| row.iter().map(|t| t.terrain.terrain_type).collect())
            .collect();

        for y in 1..self.height - 1 {
            for x in 1..self.width - 1 {
                let current = terrain_copy[y][x];

                // Check if current tile is adjacent to water
                let adjacent_water = self.is_adjacent_to_terrain(&terrain_copy, x, y, TerrainType::Water);

                if adjacent_water && current != TerrainType::Water {
                    // Create beach or riverbank based on surrounding terrain
                    let is_coastal = self.count_adjacent_terrain(&terrain_copy, x, y, TerrainType::Water) >= 2;

                    let new_terrain = if is_coastal {
                        // Coastal areas get beaches
                        if rng.gen::<f32>() < 0.7 {
                            TerrainType::Beach
                        } else {
                            TerrainType::Riverbank
                        }
                    } else {
                        // Single-water adjacent = river
                        TerrainType::Riverbank
                    };

                    // Only change Plains, Meadow to transition terrain
                    if matches!(current, TerrainType::Plains | TerrainType::Meadow) {
                        self.tiles[y][x].terrain.terrain_type = new_terrain;
                    }
                }

                // What the sea does to the ground it touches: marsh where
                // it is wet, and flats where it is not.
                //
                // This is the whole supply of salt in the world. A settlement
                // with no coast has to find a deposit in the mountains or go
                // without, which is exactly the position most inland peoples
                // were actually in.
                if self.is_adjacent_to_terrain(&terrain_copy, x, y, TerrainType::Sea) {
                    match current {
                        TerrainType::Wetland | TerrainType::Riverbank
                            if rng.gen::<f32>() < Self::HOW_OFTEN_A_MARSH_BY_THE_SEA_IS_SALT =>
                        {
                            self.tiles[y][x].terrain.terrain_type = TerrainType::SaltMarsh;
                            continue;
                        }
                        TerrainType::Desert | TerrainType::Beach | TerrainType::Plains
                            if rng.gen::<f32>()
                                < Self::HOW_OFTEN_DRY_GROUND_BY_THE_SEA_IS_A_FLAT =>
                        {
                            self.tiles[y][x].terrain.terrain_type = TerrainType::SaltFlat;
                            continue;
                        }
                        _ => {}
                    }
                }

                // Create wetlands near water in low areas
                if current == TerrainType::Plains {
                    let water_count = self.count_adjacent_terrain(&terrain_copy, x, y, TerrainType::Water);
                    let riverbank_count = self.count_adjacent_terrain(&terrain_copy, x, y, TerrainType::Riverbank);

                    if water_count + riverbank_count >= 2 && rng.gen::<f32>() < 0.4 {
                        self.tiles[y][x].terrain.terrain_type = TerrainType::Wetland;
                    }
                }
            }
        }
    }

    /// Ensure minimum terrain diversity for resource spawning
    fn ensure_terrain_diversity(&mut self, rng: &mut impl Rng) {

        // Count terrain types
        let mut terrain_counts: std::collections::BTreeMap<TerrainType, usize> = std::collections::BTreeMap::new();
        for row in &self.tiles {
            for tile in row {
                *terrain_counts.entry(tile.terrain.terrain_type).or_insert(0) += 1;
            }
        }

        let total_tiles = self.width * self.height;
        let min_percentage = 0.02; // Ensure at least 2% of each required terrain type

        // Required terrain types for resources
        let required_terrains = vec![
            TerrainType::Forest,
            TerrainType::Mountain,
            TerrainType::Plains,
            TerrainType::Hills,
            TerrainType::Meadow,
            TerrainType::Water,
        ];

        for terrain in required_terrains {
            let count = terrain_counts.get(&terrain).copied().unwrap_or(0);
            let min_count = (total_tiles as f32 * min_percentage) as usize;

            if count < min_count {
                // Add more of this terrain type
                let needed = min_count - count;
                let mut added = 0;

                for _ in 0..needed * 10 {
                    if added >= needed {
                        break;
                    }

                    let x = rng.gen_range(0..self.width);
                    let y = rng.gen_range(0..self.height);

                    // Only replace Plains to maintain variety
                    if self.tiles[y][x].terrain.terrain_type == TerrainType::Plains {
                        self.tiles[y][x].terrain.terrain_type = terrain;
                        added += 1;
                    }
                }
            }
        }
    }

    /// Check if a position is adjacent to a specific terrain type
    fn is_adjacent_to_terrain(&self, terrain_map: &[Vec<TerrainType>], x: usize, y: usize, terrain: TerrainType) -> bool {
        self.count_adjacent_terrain(terrain_map, x, y, terrain) > 0
    }

    /// Count how many adjacent tiles have a specific terrain type
    fn count_adjacent_terrain(&self, terrain_map: &[Vec<TerrainType>], x: usize, y: usize, terrain: TerrainType) -> usize {
        let mut count = 0;
        let offsets: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

        for (dx, dy) in offsets {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;

            if nx >= 0 && ny >= 0 && (nx as usize) < self.width && (ny as usize) < self.height {
                if terrain_map[ny as usize][nx as usize] == terrain {
                    count += 1;
                }
            }
        }

        count
    }

    // Simple noise function for terrain generation
    fn simple_noise(&self, x: f32, y: f32) -> f32 {
        let value = (x.sin() * 43758.5453 + y.cos() * 12345.6789).sin();
        (value + 1.0) / 2.0 // Normalize to 0-1
    }

    /// Somebody has left muck on this ground, so it is worth visiting.
    ///
    /// The register answers `Soil::has_somebody_left_something_here`, which is
    /// fouling and dropped seed and nothing else. Litter is deliberately not
    /// in it: every tile in the world is born with leaf litter on it, so a
    /// register of tiles-with-litter is a register of every tile, which costs
    /// a million set inserts a turn and saves nothing. What rots litter still
    /// sweeps the whole grid.
    ///
    /// Cheap and deliberately generous: it costs a set insert, and a tile that
    /// turns out to have gained nothing is dropped again by
    /// `forget_bare_ground`. The expensive mistake is the other one - a tile
    /// left off the list sits there for ever.
    /// Somebody has left muck on this ground.
    ///
    /// The one way to foul a tile. `Soil::somebody_voided_here` is the low
    /// level of it and knows nothing about the map it sits in, so reaching
    /// through `get_tile_mut` to call it leaves the tile off the register and
    /// it sits there smelling of nothing for ever. Anything that has a grid in
    /// its hand calls this instead.
    pub fn somebody_voided_on(&mut self, at: &Position, how_much: f32) {
        self.note_something_on(at);
        if let Some(tile) = self.get_tile_mut(at) {
            tile.soil.somebody_voided_here(how_much);
        }
    }

    pub fn note_something_on(&mut self, at: &Position) {
        if at.x < 0 || at.y < 0 {
            return;
        }
        let (x, y) = (at.x as usize, at.y as usize);
        if x < self.width && y < self.height {
            self.ground_with_something_on_it.insert((y, x));
        }
    }

    /// The ground somebody has left something on, in a fixed order.
    ///
    /// May carry a tile that has just gone bare - the pruning happens once a
    /// turn rather than on every read - so anything walking this still asks
    /// its own question of each tile.
    pub fn where_the_ground_is_doing_something(&self) -> Vec<Position> {
        self.ground_with_something_on_it
            .iter()
            .map(|(y, x)| Position::new(*x as i32, *y as i32))
            .collect()
    }

    /// Drop the tiles that have nothing on them any more.
    pub fn forget_bare_ground(&mut self) {
        let noted = std::mem::take(&mut self.ground_with_something_on_it);
        self.ground_with_something_on_it = noted
            .into_iter()
            .filter(|(y, x)| {
                self.tiles
                    .get(*y)
                    .and_then(|row| row.get(*x))
                    .is_some_and(|tile| tile.soil.has_somebody_left_something_here())
            })
            .collect();
    }

    /// How much ground is worth visiting, which is what a turn costs.
    pub fn how_much_ground_is_doing_something(&self) -> usize {
        self.ground_with_something_on_it.len()
    }

    pub fn get_tile(&self, pos: &Position) -> Option<&Tile> {
        if self.is_valid_position(pos) {
            Some(&self.tiles[pos.y as usize][pos.x as usize])
        } else {
            None
        }
    }

    // ===== The ground itself =====

    fn key(&self, at: &Position) -> Option<(usize, usize)> {
        self.is_valid_position(at)
            .then(|| (at.y as usize, at.x as usize))
    }

    /// The field on this tile, if somebody has broken it.
    pub fn field_at(&self, at: &Position) -> Option<&crate::world::soil::Field> {
        self.fields.get(&self.key(at)?)
    }

    /// The field on this tile, to be changed.
    pub fn field_at_mut(&mut self, at: &Position) -> Option<&mut crate::world::soil::Field> {
        let key = self.key(at)?;
        self.fields.get_mut(&key)
    }

    /// Every field on the map, in a fixed order.
    pub fn every_field(&self) -> impl Iterator<Item = (Position, &crate::world::soil::Field)> {
        self.fields
            .iter()
            .map(|((y, x), field)| (Position::new(*x as i32, *y as i32), field))
    }

    /// What kind of ground this is, and how good.
    ///
    /// A field's own record where there is one, and the terrain's natural
    /// ground everywhere else.
    pub fn soil_at(&self, at: &Position) -> Option<(crate::world::soil::SoilType, crate::world::soil::SoilGrade)> {
        if let Some(field) = self.field_at(at) {
            return Some((field.soil, field.grade));
        }
        let tile = self.get_tile(at)?;
        Some(crate::world::soil::SoilType::natural_to(tile.terrain.terrain_type))
    }

    /// What this ground makes of anything growing on it, against ordinary
    /// wild ground.
    ///
    /// The grade's multiplier, times `WHAT_BROKEN_GROUND_YIELDS_OVER_WILD` on
    /// broken ground, and nothing at all where nothing grows.
    pub fn what_it_yields_here(&self, at: &Position) -> f32 {
        use crate::world::soil::WHAT_BROKEN_GROUND_YIELDS_OVER_WILD;

        let Some((soil, grade)) = self.soil_at(at) else {
            return 0.0;
        };
        if !soil.grows_anything() {
            return 0.0;
        }
        let broken = self
            .get_tile(at)
            .is_some_and(|tile| tile.terrain.is_cultivated());
        grade.multiplier()
            * if broken {
                WHAT_BROKEN_GROUND_YIELDS_OVER_WILD
            } else {
                1.0
            }
    }

    /// How good this ground is, on its own: its grade, and not what breaking
    /// it would do for a crop on top. Half for ordinary ground, one for very
    /// rich, which is the scale the old fertility was read on - so a farmer
    /// who put beans in ground "under half" still does.
    pub fn how_good_the_ground_is(&self, at: &Position) -> f32 {
        match self.soil_at(at) {
            Some((soil, grade)) if soil.grows_anything() => {
                crate::world::resources::ResourceNode::WHAT_ORDINARY_WILD_GROUND_CARRIES
                    * grade.multiplier()
            }
            _ => 0.0,
        }
    }

    /// What the ground gives a wild plant, on the 0-to-1 scale the flora's
    /// growing conditions are read on.
    ///
    /// Ordinary wild ground is half, which is where the old nutrient model had
    /// open plains, and very rich ground is all of it.
    pub fn what_a_plant_gets_here(&self, at: &Position) -> f32 {
        (crate::world::resources::ResourceNode::WHAT_ORDINARY_WILD_GROUND_CARRIES
            * self.what_it_yields_here(at))
        .clamp(0.0, 1.0)
    }

    /// Whether anything will come up here at all.
    pub fn will_anything_grow_on(&self, at: &Position) -> bool {
        self.what_it_yields_here(at) > 0.0
    }

    /// Break this ground into a field, if it will be broken.
    ///
    /// Returns whether it is a field now. Ground that is already a field is
    /// worked rather than broken again; ground that cannot be tilled is left
    /// as it is.
    pub fn break_ground(&mut self, at: &Position, now: u32) -> bool {
        use crate::world::soil::Field;
        use crate::world::{Terrain, TerrainType};

        let Some(key) = self.key(at) else {
            return false;
        };
        let terrain = self.tiles[key.0][key.1].terrain.terrain_type;
        let already = terrain == TerrainType::Farmland;

        if !already && !Terrain::new(terrain).can_be_tilled() {
            return false;
        }

        self.fields
            .entry(key)
            .or_insert_with(|| Field::broken_out_of(terrain, now))
            .somebody_worked_it(now);
        self.tiles[key.0][key.1].terrain = Terrain::new(TerrainType::Farmland);
        true
    }

    /// Somebody weeded or otherwise worked the field here.
    pub fn somebody_worked_the_field(&mut self, at: &Position, now: u32) {
        if let Some(field) = self.key(at).and_then(|key| self.fields.get_mut(&key)) {
            field.somebody_worked_it(now);
        }
    }

    /// Units came off the crop on the field here. See `Field::a_crop_came_off`.
    pub fn a_crop_came_off(&mut self, at: &Position, units: u32, a_whole_crop: u32, pods: bool, now: u32) {
        if let Some(field) = self.key(at).and_then(|key| self.fields.get_mut(&key)) {
            field.a_crop_came_off(units, a_whole_crop, pods, now);
        }
    }

    /// A bean crop on the field here has come to maturity.
    pub fn a_bean_crop_came_in(&mut self, at: &Position, now: u32) {
        if let Some(field) = self.key(at).and_then(|key| self.fields.get_mut(&key)) {
            field.a_bean_crop_came_in(now);
        }
    }

    /// Somebody put muck worth `worth` on the ground here.
    ///
    /// Returns whether it will come to anything, which on wild ground it never
    /// does: only a field is built by what is carried to it.
    pub fn somebody_mucked(&mut self, at: &Position, worth: f32, now: u32) -> bool {
        self.key(at)
            .and_then(|key| self.fields.get_mut(&key))
            .is_some_and(|field| field.somebody_mucked_it(worth, now))
    }

    /// A day goes by for every field on the map.
    ///
    /// Only the fields: wild ground does not change, so this costs what the
    /// settlement farms rather than what the map is. A field that has gone
    /// back to the wild gets its terrain back, loses its weeds - a meadow
    /// cannot be any weedier than it already is - and its record.
    pub fn a_day_goes_by_for_the_fields(&mut self, now: u32) {
        use crate::world::soil::WhatBecameOfIt;

        let mut gone_wild = Vec::new();
        for (key, field) in self.fields.iter_mut() {
            if field.a_day_goes_by(now) == WhatBecameOfIt::GoneBackToTheWild {
                gone_wild.push((*key, field.was));
            }
        }

        for ((y, x), was) in gone_wild {
            self.fields.remove(&(y, x));
            let tile = &mut self.tiles[y][x];
            tile.terrain = crate::world::Terrain::new(was);
            tile.soil.weeds = 0.0;
            tile.soil.pests = 0.0;
        }
    }

    /// Let every midden on the map air out, a day's worth.
    ///
    /// Only the ground somebody has left something on: nowhere else has
    /// anything to air. This used to be the whole map every day, because it
    /// rode along with rotting the litter every tile was born with.
    pub fn a_day_of_air_for_the_middens(&mut self, precipitation: f32) {
        let noted: Vec<(usize, usize)> = self.ground_with_something_on_it.iter().copied().collect();
        for (y, x) in noted {
            let tile = &mut self.tiles[y][x];
            let humidity = crate::world::soil::Soil::humidity(tile.terrain.terrain_type, precipitation);
            tile.soil.air_out(humidity, crate::environment::seasons::ONCE_A_DAY as f32);
        }
    }

    pub fn get_tile_mut(&mut self, pos: &Position) -> Option<&mut Tile> {
        if self.is_valid_position(pos) {
            Some(&mut self.tiles[pos.y as usize][pos.x as usize])
        } else {
            None
        }
    }

    pub fn is_valid_position(&self, pos: &Position) -> bool {
        pos.x >= 0 && pos.y >= 0 && (pos.x as usize) < self.width && (pos.y as usize) < self.height
    }

    /// Find path from start to end (simple breadth-first search)
    pub fn find_path(&self, start: &Position, end: &Position) -> Option<Vec<Position>> {
        use std::collections::{BTreeMap, VecDeque};

        if !self.is_valid_position(start) || !self.is_valid_position(end) {
            return None;
        }

        if start == end {
            return Some(vec![*start]);
        }

        let mut queue = VecDeque::new();
        let mut came_from: BTreeMap<Position, Position> = BTreeMap::new();
        let mut visited = BTreeMap::new();

        queue.push_back(*start);
        visited.insert(*start, true);

        while let Some(current) = queue.pop_front() {
            if current == *end {
                // Reconstruct path
                let mut path = vec![current];
                let mut current = current;

                while let Some(&prev) = came_from.get(&current) {
                    path.push(prev);
                    current = prev;
                }

                path.reverse();
                return Some(path);
            }

            for neighbor in current.neighbors() {
                if !self.is_valid_position(&neighbor) {
                    continue;
                }

                // Check if tile is walkable
                if let Some(tile) = self.get_tile(&neighbor) {
                    if !tile.terrain.is_walkable() {
                        continue;
                    }
                }

                if !visited.contains_key(&neighbor) {
                    visited.insert(neighbor, true);
                    came_from.insert(neighbor, current);
                    queue.push_back(neighbor);
                }
            }
        }

        None // No path found
    }

    /// Find path avoiding both terrain obstacles and occupied positions
    /// Returns the next position to move to (first step of path), not the full path
    pub fn find_path_with_agents(&self, start: &Position, end: &Position, occupied_positions: &[Position]) -> Option<Position> {
        use std::collections::{BTreeMap, VecDeque};

        if !self.is_valid_position(start) || !self.is_valid_position(end) {
            return None;
        }

        if start == end {
            return None; // Already at destination
        }

        // Check if destination is walkable
        if let Some(tile) = self.get_tile(end) {
            if !tile.terrain.is_walkable() {
                return None;
            }
        }

        let mut queue = VecDeque::new();
        let mut came_from: BTreeMap<Position, Position> = BTreeMap::new();
        let mut visited = BTreeMap::new();

        queue.push_back(*start);
        visited.insert(*start, true);

        while let Some(current) = queue.pop_front() {
            if current == *end {
                // Reconstruct path and return first step
                let mut path_node = current;

                while let Some(&prev) = came_from.get(&path_node) {
                    if prev == *start {
                        // path_node is the first step from start
                        return Some(path_node);
                    }
                    path_node = prev;
                }

                return Some(current); // Shouldn't happen, but fallback
            }

            for neighbor in current.neighbors() {
                if !self.is_valid_position(&neighbor) {
                    continue;
                }

                // Skip if occupied by another agent
                if occupied_positions.contains(&neighbor) {
                    continue;
                }

                // Check if tile is walkable
                if let Some(tile) = self.get_tile(&neighbor) {
                    if !tile.terrain.is_walkable() {
                        continue;
                    }
                }

                if !visited.contains_key(&neighbor) {
                    visited.insert(neighbor, true);
                    came_from.insert(neighbor, current);
                    queue.push_back(neighbor);
                }
            }
        }

        None // No path found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_distance() {
        let p1 = Position::new(0, 0);
        let p2 = Position::new(3, 4);

        assert_eq!(p1.distance_to(&p2), 7); // Manhattan distance
        assert!((p1.euclidean_distance_to(&p2) - 5.0).abs() < 0.001); // Euclidean distance
    }

    #[test]
    fn test_position_neighbors() {
        let pos = Position::new(5, 5);
        let neighbors = pos.neighbors();

        assert_eq!(neighbors.len(), 4);
        assert!(neighbors.contains(&Position::new(6, 5)));
        assert!(neighbors.contains(&Position::new(4, 5)));
        assert!(neighbors.contains(&Position::new(5, 6)));
        assert!(neighbors.contains(&Position::new(5, 4)));
    }

    #[test]
    fn test_grid_creation() {
        let grid = Grid::new(10, 10);
        assert_eq!(grid.width, 10);
        assert_eq!(grid.height, 10);
        assert_eq!(grid.tiles.len(), 10);
        assert_eq!(grid.tiles[0].len(), 10);
    }

    #[test]
    fn test_grid_valid_position() {
        let grid = Grid::new(10, 10);

        assert!(grid.is_valid_position(&Position::new(0, 0)));
        assert!(grid.is_valid_position(&Position::new(9, 9)));
        assert!(!grid.is_valid_position(&Position::new(-1, 0)));
        assert!(!grid.is_valid_position(&Position::new(0, -1)));
        assert!(!grid.is_valid_position(&Position::new(10, 0)));
        assert!(!grid.is_valid_position(&Position::new(0, 10)));
    }

    #[test]
    fn test_grid_get_tile() {
        let grid = Grid::new(10, 10);
        let pos = Position::new(5, 5);

        assert!(grid.get_tile(&pos).is_some());
        assert!(grid.get_tile(&Position::new(-1, 0)).is_none());
    }
}
