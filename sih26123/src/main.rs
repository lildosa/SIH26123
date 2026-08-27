use sih26123::world::GridMap;

#[tokio::main]
async fn main() {
    let grid = GridMap::generate_warehouse(15, 15, 3);
    println!("SIH26123 AMR Fleet Coordination Engine (Grid: {}x{})", grid.width, grid.height);
}
