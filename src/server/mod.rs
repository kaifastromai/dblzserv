use std::sync::Arc;

use ah::Context;
use bson::oid::ObjectId;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tokio_stream::Stream;
use tokio_stream::StreamExt;
use tracing::info;
const BLITZ_PILE: &str = "BLITZ_PILE";
const POST_PILE: &str = "POST_PILE";
const AVAILABLE_PILE: &str = "AVAILABLE_PILE";

use crate::proto;
use crate::proto::*;
use crate::GameState;
use anyhow as ah;

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
    pub client_event_channels: Vec<
        Option<(
            tokio::sync::mpsc::Sender<proto::ServerEvent>,
            JoinHandle<()>,
        )>,
    >,
}
impl Session {
    pub fn start_game(&mut self, rq: StartGameEvent) -> anyhow::Result<()> {
        let player = rq.player.unwrap();
        if !player.is_session_admin {
            return Err(ah::anyhow!("Player is not admin"));
        }
        if !self.is_joinable {
            return Err(ah::anyhow!("Session is already in game"));
        }
        self.is_joinable = false;
        self.game_state = Some(
            GameState::new(self.players.len() as u32, rq.prefs.unwrap())
                .with_context(|| "Failed to create game state")?,
        );
        Ok(())
    }
}
pub struct Server {
    sessions: Arc<DashMap<String, Session>>,
}
impl Server {
    pub fn new() -> Self {
        Server {
            sessions: Arc::new(DashMap::new()),
        }
    }
    pub fn create_session(&self, rq: proto::StartSessionRq) -> anyhow::Result<proto::Player> {
        let session_id = ObjectId::new().to_hex();
        let player_id = ObjectId::new().to_hex();
        let player = Player {
            session_id: session_id.clone(),
            player_game_id: 0,
            username: rq.username,
            face_image_id: rq.face_image_id,
            is_session_admin: true,
        };

        let session = Session {
            id: session_id.clone(),
            is_joinable: true,
            game_state: None,
            players: vec![player.clone()],
            client_event_channels: vec![None],
        };
        self.sessions.insert(session_id.clone(), session);
        Ok(player)
    }
    pub fn join_session(&self, rq: JoinSessionRq) -> ah::Result<Player> {
        let session_id = rq.session_id.clone();
        let mut session = self
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| ah::anyhow!("Session not found"))?;
        if !session.is_joinable {
            return Err(ah::anyhow!("Session is not joinable"));
        }
        let player_game_id = session.players.len() as u32;
        let player = Player {
            session_id: session_id.clone(),
            player_game_id,
            username: rq.username,
            face_image_id: rq.face_image_id,
            is_session_admin: false,
        };
        session.players.push(player.clone());
        Ok(player)
    }
    async fn process_client_events(
        &self,
        session_id: String,
        player_id: u32,
        mut rx: tokio::sync::mpsc::Receiver<proto::ClientEvent>,
    ) -> tokio::task::JoinHandle<()> {
        let session_arc = self.sessions.clone();
        tokio::spawn(async move {
            while let Some(ClientEvent {
                player_id,
                event_id,
                event: Some(e),
            }) = rx.recv().await
            {
                match e {
                    client_event::Event::Play(p) => {
                        tracing::info!(player_id, event_id, "Play event received. Event: {p:?}");
                    }
                    client_event::Event::StaticEvent(s) => {
                        let client_game_state_action: ClientGameStateAction = s.try_into().unwrap();
                        tracing::info!(
                            player_id,
                            event_id,
                            "Static event received. Event: {client_game_state_action:?}"
                        );
                    }
                    client_event::Event::OpenStream(_) => {
                        tracing::warn!("OpenStream event received. This should not happen");
                    }
                    client_event::Event::StartGame(s) => {
                        tracing::info!(
                            player_id,
                            event_id,
                            "StartGame event received. Event: {s:?}"
                        );
                        if player_id != 0 {
                            //the game can only be started by player 0 (admin)
                            tracing::warn!("Player {} tried to start game", player_id);
                            continue;
                        }
                        let mut session = session_arc.get_mut(&session_id).unwrap();
                        session
                            .start_game(s.clone())
                            .with_context(|| "Failed to start game")
                            .unwrap();
                        let e = server_event::Event::StartGame(ServerStartGameEvent {
                            prefs: s.prefs.clone(),
                        });
                        Self::broadcast_event(e, &session, player_id, event_id)
                            .await
                            .with_context(|| "Failed to broadcast event")
                            .unwrap();

                        //tell all clients that the game has started
                    }
                }
            }
        })
    }
    pub async fn broadcast_event(
        event: server_event::Event,
        session: &Session,
        player_id: u32,
        event_id: u32,
    ) -> anyhow::Result<()> {
        for player in session.players.iter() {
            if let Some((tx, _)) =
                session.client_event_channels[player.player_game_id as usize].as_ref()
            {
                if player.player_game_id == player_id {
                    //send acknoledgement to sender
                    let ack = Acknowledge { event_id };
                    tx.send(ServerEvent {
                        event: Some(server_event::Event::Acknowledge(ack)),
                    })
                    .await
                    .unwrap();
                }
                tx.send(ServerEvent {
                    event: Some(event.clone()),
                })
                .await
                .unwrap();
            }
        }
        Ok(())
    }
    pub async fn open_event_stream(
        &self,
        mut rq: impl Stream<Item = ClientEvent> + std::marker::Unpin,
    ) -> anyhow::Result<tokio::sync::mpsc::Receiver<ServerEvent>> {
        let (server_tx, server_rx) = tokio::sync::mpsc::channel(100);
        let (_, client_rx) = tokio::sync::mpsc::channel(100);
        if let Some(first_mes) = rq.next().await {
            if let ClientEvent {
                player_id: id,
                event: Some(client_event::Event::OpenStream(ClientInitOpenStream { session_id })),
                ..
            } = first_mes
            {
                //add this channel to the sessions list of channels
                let mut session = self
                    .sessions
                    .get_mut(&session_id)
                    .ok_or_else(|| ah::anyhow!("Session not found"))?;
                let channel = session
                    .client_event_channels
                    .get_mut(id as usize)
                    .with_context(|| format!("Player not found: {}", id))?;
                if channel.is_some() {
                    return Err(ah::anyhow!("Channel already open"));
                } else {
                    let join = self.process_client_events(session_id, id, client_rx).await;
                    session.client_event_channels[id as usize] = Some((server_tx, join));
                }
            } else {
                tracing::error!("Fatal error: First message is not OpenStream");
                anyhow::bail!("Fatal error: First message is not OpenStream")
            }
        } else {
            tracing::error!("Fatal error: No first message");
            anyhow::bail!("Fatal error: No first message")
        }

        Ok(server_rx)
    }
}
#[tonic::async_trait]
impl proto::session_service_server::SessionService for Server {
    async fn get_active_sessions(
        &self,
        request: tonic::Request<()>,
    ) -> std::result::Result<tonic::Response<SessionRes>, tonic::Status> {
        todo!()
    }
    /// Join an active session
    async fn join_session(
        &self,
        request: tonic::Request<JoinSessionRq>,
    ) -> std::result::Result<tonic::Response<Player>, tonic::Status> {
        let rq = request.into_inner();
        let player = self.join_session(rq).into_tonic_status()?;
        Ok(tonic::Response::new(player))
    }
    /// Start a new session that other players can join. The player who starts the session is the session admin. It then returns a Player message
    /// with the player_id field set to the player's unique identifier
    async fn start_session(
        &self,
        request: tonic::Request<StartSessionRq>,
    ) -> std::result::Result<tonic::Response<Player>, tonic::Status> {
        let rq = request.into_inner();
        let player = self.create_session(rq).into_tonic_status()?;
        Ok(tonic::Response::new(player))
    }
}
pub struct TonicStatus(tonic::Status);
impl From<tonic::Status> for TonicStatus {
    fn from(s: tonic::Status) -> Self {
        TonicStatus(s)
    }
}
impl From<ah::Error> for TonicStatus {
    fn from(e: ah::Error) -> Self {
        TonicStatus(tonic::Status::new(tonic::Code::Internal, format!("{}", e)))
    }
}
pub trait AnyhowIntoTonicStatus<T> {
    fn into_tonic_status(self) -> Result<T, tonic::Status>;
}
impl<T> AnyhowIntoTonicStatus<T> for anyhow::Result<T> {
    fn into_tonic_status(self) -> Result<T, tonic::Status> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => {
                tracing::error!("Error: {}", e);
                Err(tonic::Status::new(tonic::Code::Internal, format!("{}", e)))
            }
        }
    }
}
