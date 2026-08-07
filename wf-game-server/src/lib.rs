use std::collections::{HashMap, VecDeque};

use rocket::post;

pub mod game_start;

pub struct WfGame {
    games: HashMap<usize, Match>,
    queued: VecDeque<QueuedPlayer>,
}

pub struct Match {
    player_1: Player,
    player_2: Player,

    is_player_1_turn: bool,
    n_turns: usize,

    map_desc: MapDesc
}

pub struct MapDesc {
    rows: usize,
    columns: usize,

    boats: Vec<Boat>
}

pub struct Player {
    name: String, 
}

pub struct PlayerMap {
    rows: usize,
    columns: usize,

    boats: Vec<Vec<BoatPart>>
}

pub enum BoatPart {
    Leader(usize, usize),
    Regular(usize, usize), // leader coords 
}

pub struct QueuedPlayer {
    name: String,
    map_desc: MapDesc
}

pub struct Boat {
    rows: usize,
    cols: usize,
}