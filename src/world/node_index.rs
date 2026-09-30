// src/world/node_index.rs
//! Where the nodes are, so that nobody has to read the whole map to find one.
//!
//! Every question an agent asked of the ground - what is on this tile, what
//! food is within a walk, which reach of river is worth standing in - was
//! answered by reading every node on the map and throwing away the ones too
//! far off. On the maps the model was written for that is a thousand nodes and
//! nobody noticed. On a map sixteen times the size it is sixteen times the
//! reading for exactly the same answers, per agent, several times a turn: a
//! dozen people on an 800-cell map cost four and a half seconds a day, nearly
//! all of it reading ground none of them could see (ISSUES_FOUND #248).
//!
//! So the world keeps the nodes filed by the patch of map they stand on, and
//! a question about somewhere reads only the patches around it. The patches
//! hold node numbers in list order, and whatever is read out comes back in
//! list order, so every answer - including which of two equally good things
//! comes first - is exactly the answer the whole-map read gave.
//!
//! The file has to agree with the list it files. Everything that adds or
//! takes away a node in a running world goes through [`super::World`]'s
//! `put_a_node_down` and `take_a_node_up`, which keep it up to date, and the
//! world refiles everything once a turn besides. Anything that changes the
//! list behind its back - a test pushing a node by hand - leaves the file
//! counting a different number of nodes from the list, or a different last
//! one, and until the next refiling every question falls back to reading the
//! whole list: slower, and still right.

use super::ResourceNode;

/// The side of a patch, in cells.
///
/// Small enough that a question about the twenty-five cells an agent forages
/// over reads not much more than the ground it asks about; large enough that
/// the patches of an 1,600-cell map are forty thousand short lists rather than
/// millions.
pub const A_PATCH_OF_NODES: i32 = 8;

/// Node numbers by the patch of map they stand on.
#[derive(Debug, Clone, Default)]
pub struct WhereTheNodesAre {
    across: usize,
    down: usize,
    patches: Vec<Vec<u32>>,
    /// Anything standing off the edge of the map, which a well-made world
    /// never has; read by every question so that nothing can be missed.
    off_the_map: Vec<u32>,
    /// How many nodes the list held when this was filed.
    filed: usize,
    /// And where the last of them stood: one taken away and one added by
    /// hand leaves the count as it was, and very seldom leaves this.
    last: Option<(i32, i32)>,
}

impl WhereTheNodesAre {
    /// File every node in `nodes`, on a map this size.
    pub fn file(nodes: &[ResourceNode], width: usize, height: usize) -> Self {
        let across = width.div_ceil(A_PATCH_OF_NODES as usize).max(1);
        let down = height.div_ceil(A_PATCH_OF_NODES as usize).max(1);
        let mut filing = Self {
            across,
            down,
            patches: vec![Vec::new(); across * down],
            off_the_map: Vec::new(),
            filed: 0,
            last: None,
        };

        for node in nodes {
            filing.another(node);
        }

        filing
    }

    /// Whether this still describes `nodes`.
    pub fn is_it_up_to_date(&self, nodes: &[ResourceNode]) -> bool {
        !self.patches.is_empty()
            && self.filed == nodes.len()
            && self.last == nodes.last().map(|node| (node.position.x, node.position.y))
    }

    /// File one more node, the next in the list.
    pub fn another(&mut self, node: &ResourceNode) {
        let number = self.filed as u32;
        match self.patch_of(node.position.x, node.position.y) {
            Some(patch) => self.patches[patch].push(number),
            None => self.off_the_map.push(number),
        }
        self.filed += 1;
        self.last = Some((node.position.x, node.position.y));
    }

    fn patch_of(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 {
            return None;
        }
        let (across, down) = ((x / A_PATCH_OF_NODES) as usize, (y / A_PATCH_OF_NODES) as usize);
        (across < self.across && down < self.down).then_some(down * self.across + across)
    }

    /// The numbers of every node within `reach` cells either way of
    /// `(x, y)`, in list order - and possibly a few further off, from the
    /// corners of the patches read, which the asker's own distance rule
    /// throws out.
    pub fn near(&self, x: i32, y: i32, reach: i32) -> Vec<usize> {
        let reach = reach.max(0);
        let clamp_across = |v: i32| (v.max(0) / A_PATCH_OF_NODES).min(self.across as i32 - 1) as usize;
        let clamp_down = |v: i32| (v.max(0) / A_PATCH_OF_NODES).min(self.down as i32 - 1) as usize;

        let mut found: Vec<usize> = self.off_the_map.iter().map(|&n| n as usize).collect();

        let (west, east) = (x.saturating_sub(reach), x.saturating_add(reach));
        let (north, south) = (y.saturating_sub(reach), y.saturating_add(reach));

        // Nothing on the map is that far out, whichever way.
        let clear_of_it = east < 0
            || south < 0
            || west >= self.across as i32 * A_PATCH_OF_NODES
            || north >= self.down as i32 * A_PATCH_OF_NODES;
        if !clear_of_it {
            for down in clamp_down(north)..=clamp_down(south) {
                for across in clamp_across(west)..=clamp_across(east) {
                    found.extend(
                        self.patches[down * self.across + across]
                            .iter()
                            .map(|&n| n as usize),
                    );
                }
            }
        }

        found.sort_unstable();
        found
    }
}
