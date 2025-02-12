// position management module for tracking multiple positions

#[derive(Debug, Clone)]
pub struct PositionManager {
    pub max_positions: usize,     // maximum number of positions allowed per side
    pub open_longs: usize,        // number of currently open long positions
    pub open_shorts: usize,       // number of currently open short positions
}

impl PositionManager {
    pub fn new(max_positions: usize) -> Self {
        PositionManager {
            max_positions,
            open_longs: 0,
            open_shorts: 0,
        }
    }

    // check if we can open a new long position
    pub fn can_open_long(&self) -> bool {
        self.open_longs < self.max_positions
    }

    // check if we can open a new short position
    pub fn can_open_short(&self) -> bool {
        self.open_shorts < self.max_positions
    }

    // register a new position
    pub fn register_position(&mut self, size: f64) {
        if size > 0.0 {
            self.open_longs += 1;
        } else {
            self.open_shorts += 1;
        }
    }

    // close a position
    pub fn close_position(&mut self, size: f64) {
        if size > 0.0 {
            self.open_longs = self.open_longs.saturating_sub(1);
        } else {
            self.open_shorts = self.open_shorts.saturating_sub(1);
        }
    }

    // get total number of open positions
    pub fn total_positions(&self) -> usize {
        self.open_longs + self.open_shorts
    }

    // reset all position counters
    pub fn reset(&mut self) {
        self.open_longs = 0;
        self.open_shorts = 0;
    }
}