use actix_web::post;
use actix_web::web;
use actix_web::Responder;
use actix_web::Result;
use bson::oid::ObjectId;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::info;
const BLITZ_PILE: &str = "BLITZ_PILE";
const POST_PILE: &str = "POST_PILE";
const AVAILABLE_PILE: &str = "AVAILABLE_PILE";

use crate::GameState;
use anyhow as ah;

///Since the clients don't need to render every pile of cards,
/// send only the topmost card and the size of the pile
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PileTop {
    pub top_card: u32,
    pub card: u32,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetArenaSnapshotRq {
    arena_hash: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetArenaSnapshotRs {
    arena: Option<Vec<PileTop>>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]

pub struct GetAllPlayersSnapshotRq {
    players_hash: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]

pub struct GetAllPlayersSnapshotRs {
    pub is_new: bool,
    pub player_piles: Option<PlayerSnapshot>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]

pub struct PlayerSnapshot {
    blitz_pile: PileTop,
    post_pile: Vec<PileTop>,
    available_pile: PileTop,
}
#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]

pub struct Player {
    pub id: String,
    pub name: String,
    pub face_id: u32,
    pub is_session_admin: bool,
}

///Creates a new session with the passed player_name marked as the session admin
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]

pub struct CreateSessionRq {
    pub player_name: String,
    pub face_id: u32,
}
///Joins an already active session
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]

pub struct JoinSessionRq {
    pub session_id: String,
    pub player_name: String,
    pub face_id: u32,
}

///A join session rs that contains the id of the player that joined and the session id
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]

pub struct JoinSessionRs {
    pub session_id: String,
    pub player: Player,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EPile {
    Blitz,
    Post,
    Available,
    Arena,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]

pub struct PlayerPlay {
    from: EPile,
    to: EPile,
    from_index: Option<u32>,
    to_index: Option<u32>,
}
///A request to play a card
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]

pub struct PlayCardRq {
    pub player_id: String,
    pub session_id: String,
    pub play: PlayerPlay,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]

///A Json response to a request to show sessions
pub struct SessionRs {
    pub session_id: String,

    pub is_joinable: bool,
    pub is_active: bool,
    pub players: Vec<Player>,
}

impl SessionRs {
    pub fn from_session(session: &Session) -> Self {
        SessionRs {
            session_id: session.id.clone(),
            is_joinable: session.is_joinable,
            is_active: session.game_state.is_some(),
            players: session.players.clone(),
        }
    }
}
///A session that is either currently waiting to be joined or is already being played
pub struct Session {
    pub id: String,
    ///whether or not this session can be joined by others. When not true,
    ///  this game is either already being  or about to be played
    pub is_joinable: bool,
    pub game_state: Option<GameState>,
    pub players: Vec<Player>,
}

pub struct Server {
    sessions: DashMap<String, Session>,
}
impl Server {
    pub fn new() -> Self {
        Server {
            sessions: DashMap::new(),
        }
    }
    pub fn create_session(&self, rq: CreateSessionRq) -> Result<JoinSessionRs> {
        let session_id = ObjectId::new().to_hex();
        let player_id = ObjectId::new().to_hex();
        let player = Player {
            id: player_id.clone(),
            name: rq.player_name,
            face_id: rq.face_id,
            is_session_admin: true,
        };
        let session = Session {
            id: session_id.clone(),
            is_joinable: true,
            game_state: None,
            players: vec![player.clone()],
        };
        self.sessions.insert(session_id.clone(), session);
        Ok(JoinSessionRs { player, session_id })
    }
    pub fn join_session(&self, rq: JoinSessionRq) -> ah::Result<JoinSessionRs> {
        let session_id = rq.session_id.clone();
        let player_id = ObjectId::new().to_hex();
        let player = Player {
            id: player_id.clone(),
            name: rq.player_name,
            face_id: rq.face_id,
            is_session_admin: false,
        };
        let mut session = self
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| ah::anyhow!("Session not found"))?;
        if !session.is_joinable {
            return Err(ah::anyhow!("Session is not joinable"));
        }
        session.players.push(player.clone());
        Ok(JoinSessionRs { player, session_id })
    }
}

//create the actix web server
#[post("/create_session")]
async fn create_session(
    rq: web::Json<CreateSessionRq>,
    data: web::Data<Server>,
) -> actix_web::Result<impl Responder> {
    info!("Trying to create session for player {}", rq.player_name);
    let rq = rq.into_inner();
    let res = data.create_session(rq);
    res.map(|res| Ok::<_, actix_web::Error>(web::Json(res)))
}
#[post("/join_session")]
async fn join_session(
    web::Json(rq): web::Json<JoinSessionRq>,
    data: web::Data<Server>,
) -> actix_web::Result<impl Responder> {
    info!(
        "Trying to join player {} to session {}",
        rq.player_name, rq.session_id
    );
    let res = data.join_session(rq);
    let res = res
        .map(|res| Ok::<_, actix_web::Error>(web::Json(res)))
        .map_err(|e| {
            info!("Error joining session: {}", e);
            actix_web::error::ErrorInternalServerError(e)
        });
    info!("Success");
    res
}

#[post("/show_sessions")]
async fn show_sessions(data: web::Data<Server>) -> impl Responder {
    let sessions = data
        .sessions
        .iter()
        .map(|s| SessionRs::from_session(&s))
        .collect::<Vec<_>>();
    web::Json(sessions)
}
