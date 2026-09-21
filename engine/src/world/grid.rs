use serde::{Deserialize, Serialize};

/// A single cell in the warehouse grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Cell {
    Free,
    Wall,
    Pickup(u32),
    Dropoff(u32),
}

/// (col, row) position on the grid. 0-indexed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct Pos {
    pub x: usize,
    pub y: usize,
}

impl Pos {
    pub fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }

    pub fn manhattan_distance(&self, other: &Pos) -> usize {
        self.x.abs_diff(other.x) + self.y.abs_diff(other.y)
    }
}

/// The warehouse map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridMap {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<Cell>>,
}

impl GridMap {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![vec![Cell::Free; width]; height],
        }
    }

    pub fn set_cell(&mut self, pos: Pos, cell: Cell) {
        assert!(self.in_bounds(pos), "Pos {:?} out of bounds", pos);
        self.cells[pos.y][pos.x] = cell;
    }

    pub fn get_cell(&self, pos: Pos) -> Cell {
        assert!(self.in_bounds(pos), "Pos {:?} out of bounds", pos);
        self.cells[pos.y][pos.x]
    }

    pub fn is_walkable(&self, pos: Pos) -> bool {
        if !self.in_bounds(pos) {
            return false;
        }
        matches!(
            self.cells[pos.y][pos.x],
            Cell::Free | Cell::Pickup(_) | Cell::Dropoff(_)
        )
    }

    pub fn in_bounds(&self, pos: Pos) -> bool {
        pos.x < self.width && pos.y < self.height
    }

    pub fn neighbors(&self, pos: Pos) -> Vec<Pos> {
        let mut nbrs = Vec::with_capacity(4);
        // Cardinal moves only (no diagonals)
        if pos.x > 0 {
            let left = Pos::new(pos.x - 1, pos.y);
            if self.is_walkable(left) {
                nbrs.push(left);
            }
        }
        if pos.x + 1 < self.width {
            let right = Pos::new(pos.x + 1, pos.y);
            if self.is_walkable(right) {
                nbrs.push(right);
            }
        }
        if pos.y > 0 {
            let up = Pos::new(pos.x, pos.y - 1);
            if self.is_walkable(up) {
                nbrs.push(up);
            }
        }
        if pos.y + 1 < self.height {
            let down = Pos::new(pos.x, pos.y + 1);
            if self.is_walkable(down) {
                nbrs.push(down);
            }
        }
        nbrs
    }

    pub fn generate_warehouse(width: usize, height: usize, aisle_spacing: usize) -> Self {
        let mut grid = Self::new(width, height);
        if aisle_spacing == 0 || width < 3 || height < 3 {
            return grid;
        }

        // Horizontal shelf walls spaced aisle_spacing apart
        let spacing = aisle_spacing.max(2);
        for y in (spacing..height.saturating_sub(1)).step_by(spacing) {
            for x in 1..width.saturating_sub(1) {
                // Leave cross-aisle gap in the center and at edges
                let center = width / 2;
                if x == center || x == center.saturating_sub(1) {
                    continue;
                }
                grid.set_cell(Pos::new(x, y), Cell::Wall);
            }
        }

        // Place 2 Pickup and 2 Dropoff stations at designated areas
        grid.set_cell(Pos::new(0, 0), Cell::Pickup(1));
        grid.set_cell(Pos::new(0, height - 1), Cell::Pickup(2));
        grid.set_cell(Pos::new(width - 1, 0), Cell::Dropoff(1));
        grid.set_cell(Pos::new(width - 1, height - 1), Cell::Dropoff(2));

        grid
    }
}
