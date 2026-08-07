use rocket::{State, post};

use crate::{MapDesc, WfGame};

pub struct GameStartData {
    map_desc: MapDesc,

}


#[post("/game/start")]
pub fn start(state: &State<WfGame>) {

}