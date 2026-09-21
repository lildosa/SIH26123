use crate::world::Pos;

/// Local sensing interface. Robots observe the environment through this trait.
/// The simulator provides a view of the ground-truth grid with injected obstacles,
/// but robots cannot directly mutate the ground truth.
pub trait Environment: Send + Sync + 'static {
    /// Returns the set of cells the robot can observe as blocked from its
    /// current position within `sense_radius` Manhattan distance.
    fn sense_obstacles(&self, robot_pos: Pos, sense_radius: usize) -> Vec<Pos>;

    /// Returns true if a specific cell is currently blocked.
    fn is_blocked(&self, pos: Pos) -> bool;
}
