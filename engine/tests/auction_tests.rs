use sih26123::auction::{compute_bid_cost, Auction, AuctionConfig};
use sih26123::protocol::BidMsg;
use sih26123::world::Pos;

#[test]
fn test_lowest_bid_wins() {
    let mut auction = Auction::new(101, Pos::new(0, 0), Pos::new(5, 5), 0, 5);
    auction.add_bid(1, &BidMsg { task_id: 101, cost: 5.0 });
    auction.add_bid(2, &BidMsg { task_id: 101, cost: 3.0 });
    auction.add_bid(3, &BidMsg { task_id: 101, cost: 7.0 });

    let award = auction.determine_winner();
    assert!(award.is_some());
    let award = award.unwrap();
    assert_eq!(award.winner_id, 2, "Robot 2 with cost 3.0 should win");
    assert_eq!(award.task_id, 101);
}

#[test]
fn test_tie_broken_by_id() {
    let mut auction = Auction::new(102, Pos::new(0, 0), Pos::new(5, 5), 0, 5);
    // Both bids have cost 4.0
    auction.add_bid(5, &BidMsg { task_id: 102, cost: 4.0 });
    auction.add_bid(2, &BidMsg { task_id: 102, cost: 4.0 });

    let award = auction.determine_winner();
    assert!(award.is_some());
    let award = award.unwrap();
    assert_eq!(award.winner_id, 2, "Robot 2 with lower ID should win tiebreak");
}

#[test]
fn test_no_bids_no_winner() {
    let auction = Auction::new(103, Pos::new(0, 0), Pos::new(5, 5), 0, 5);
    assert!(auction.determine_winner().is_none());
}

#[test]
fn test_bid_cost_idle_robot() {
    let config = AuctionConfig::default();
    // Robot at (0,0), battery 0.8, pickup (5,0), dropoff (5,5), congestion 0.0, task_remaining 0, no deadline
    // travel = (5-0) + (5-0) = 10
    // cost = 1.0*10 + 0.3*0*10 + 0.5*(1.0-0.8) + 0.8*0 + 0.4*0 = 10.0 + 0.1 = 10.1
    let cost = compute_bid_cost(
        &config,
        Pos::new(0, 0),
        0.8,
        Pos::new(5, 0),
        Pos::new(5, 5),
        0.0,
        0,
        None,
        0,
    );

    assert!(
        (cost - 10.1).abs() < 1e-6,
        "Expected cost ~10.1, got {}",
        cost
    );
}

#[test]
fn test_congestion_increases_cost() {
    let config = AuctionConfig::default();
    // Same as test 4, but congestion = 0.5
    // cost = 10.0 + 0.3*0.5*10 + 0.1 = 10.0 + 1.5 + 0.1 = 11.6
    let cost = compute_bid_cost(
        &config,
        Pos::new(0, 0),
        0.8,
        Pos::new(5, 0),
        Pos::new(5, 5),
        0.5,
        0,
        None,
        0,
    );

    assert!(
        (cost - 11.6).abs() < 1e-6,
        "Expected cost ~11.6, got {}",
        cost
    );
}

#[test]
fn test_busy_robot_penalized() {
    let config = AuctionConfig::default();
    // Same as test 4, but current_task_remaining = 15 steps
    // delay_cost = 0.8 * 15 = 12.0
    // total = 10.1 + 12.0 = 22.1
    let cost = compute_bid_cost(
        &config,
        Pos::new(0, 0),
        0.8,
        Pos::new(5, 0),
        Pos::new(5, 5),
        0.0,
        15,
        None,
        0,
    );

    assert!(
        (cost - 22.1).abs() < 1e-6,
        "Expected cost ~22.1, got {}",
        cost
    );
}
