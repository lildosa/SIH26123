pub mod world;
pub mod protocol;
pub mod node;
pub mod planner;
pub mod negotiator;
pub mod auction;
pub mod network;
pub mod baseline;
pub mod metrics;
pub mod dashboard;
pub mod sim;

#[tokio::main]
async fn main() {
    println!("SIH26123 AMR Fleet Coordination Engine (Phase 1 Scaffold Ready)");
}
