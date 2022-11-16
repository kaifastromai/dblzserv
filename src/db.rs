//!A simple connection to a mongo databse for session tracking and allowings players to be able to search for games

use bson::oid::ObjectId;

pub static MDB_SESSIONS_COLLECTION: &str = "sessions";

pub struct Session {
    ///the session id. Used to join
    _id: mongodb::bson::oid::ObjectId,
    ///whether or not other people can join this session
    is_accepting_players: bool,
    pub players: Vec<ObjectId>,
}
