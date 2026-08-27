use crate::planner::{plan, SpaceTimeConstraints};
use crate::protocol::{RobotId, Tick};
use crate::world::{GridMap, Pos};
use std::collections::BinaryHeap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CBSConstraint {
    Vertex(RobotId, Pos, Tick),
    Edge(RobotId, Pos, Pos, Tick),
}

#[derive(Clone)]
struct CBSNode {
    constraints: Vec<CBSConstraint>,
    paths: Vec<Vec<(Pos, Tick)>>,
    cost: usize,
}

impl PartialEq for CBSNode {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
    }
}
impl Eq for CBSNode {}
impl PartialOrd for CBSNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for CBSNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.cost.cmp(&self.cost) // Min-heap
    }
}

/// Conflict-Based Search (CBS) for optimal centralized multi-agent path finding.
pub fn cbs_plan(
    grid: &GridMap,
    agents: &[(RobotId, Pos, Pos)], // (id, start, goal)
    start_tick: Tick,
) -> Option<Vec<Vec<(Pos, Tick)>>> {
    if agents.is_empty() {
        return Some(Vec::new());
    }

    let mut root_paths = Vec::new();
    let empty_constraints = SpaceTimeConstraints::default();

    for &(id, start, goal) in agents {
        let p = plan(grid, id, start, start_tick, goal, &empty_constraints, 200)?;
        root_paths.push(p);
    }

    let initial_cost = root_paths.iter().map(|p| p.len()).sum();
    let root = CBSNode {
        constraints: Vec::new(),
        paths: root_paths,
        cost: initial_cost,
    };

    let mut open = BinaryHeap::new();
    open.push(root);

    let mut iterations = 0;
    while let Some(curr) = open.pop() {
        iterations += 1;
        if iterations > 3000 {
            break;
        }

        // Find first conflict between any two agents
        let mut first_conflict = None;
        let num_agents = curr.paths.len();

        'outer: for i in 0..num_agents {
            for j in (i + 1)..num_agents {
                let p_i = &curr.paths[i];
                let p_j = &curr.paths[j];

                // Check vertex conflict
                for &(pos_i, tick_i) in p_i {
                    for &(pos_j, tick_j) in p_j {
                        if pos_i == pos_j && tick_i == tick_j {
                            first_conflict = Some((
                                CBSConstraint::Vertex(agents[i].0, pos_i, tick_i),
                                CBSConstraint::Vertex(agents[j].0, pos_j, tick_j),
                            ));
                            break 'outer;
                        }
                    }
                }

                // Check edge swap conflict
                for w_i in p_i.windows(2) {
                    for w_j in p_j.windows(2) {
                        let (from_i, t_i) = w_i[0];
                        let (to_i, _) = w_i[1];
                        let (from_j, t_j) = w_j[0];
                        let (to_j, _) = w_j[1];

                        if t_i == t_j && from_i == to_j && to_i == from_j {
                            first_conflict = Some((
                                CBSConstraint::Edge(agents[i].0, from_i, to_i, t_i),
                                CBSConstraint::Edge(agents[j].0, from_j, to_j, t_j),
                            ));
                            break 'outer;
                        }
                    }
                }
            }
        }

        if let Some((c1, c2)) = first_conflict {
            for &branch_constraint in &[&c1, &c2] {
                let mut new_constraints = curr.constraints.clone();
                new_constraints.push(branch_constraint.clone());

                let branch_agent_id = match branch_constraint {
                    CBSConstraint::Vertex(id, _, _) => *id,
                    CBSConstraint::Edge(id, _, _, _) => *id,
                };

                let mut new_paths = curr.paths.clone();
                let agent_idx = agents.iter().position(|&(id, _, _)| id == branch_agent_id).unwrap();
                let (id, start, goal) = agents[agent_idx];

                let mut st_constraints = SpaceTimeConstraints::default();
                for c in &new_constraints {
                    match c {
                        CBSConstraint::Vertex(c_id, c_pos, c_tick) if *c_id == id => {
                            st_constraints.forbidden_cells.insert((*c_pos, *c_tick));
                        }
                        CBSConstraint::Edge(c_id, from, to, t) if *c_id == id => {
                            st_constraints.forbidden_edges.insert((*from, *to, *t));
                        }
                        _ => {}
                    }
                }

                if let Some(new_p) = plan(grid, id, start, start_tick, goal, &st_constraints, 200) {
                    new_paths[agent_idx] = new_p;
                    let cost = new_paths.iter().map(|p| p.len()).sum();
                    open.push(CBSNode {
                        constraints: new_constraints,
                        paths: new_paths,
                        cost,
                    });
                }
            }
        } else {
            return Some(curr.paths);
        }
    }

    None
}
