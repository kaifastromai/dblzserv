use std::sync::Arc;

use ah::Context;
use bson::oid::ObjectId;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tokio_stream::Stream;
use tokio_stream::StreamExt;
use tonic::client;
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
type ServerEventChannelTx = flume::Sender<tonic::Result<proto::ServerEvent>>;
type EventChannelRx = flume::Receiver<tonic::Result<ServerEvent>>;

///A session that is either currently waiting to be joined or is already being played
pub struct Session {
    pub id: String,
    ///whether or not this session can be joined by others. When not true,
    ///  this game is either already being  or about to be played
    pub is_joinable: bool,
    pub game_state: Option<GameState>,
    pub players: Vec<Player>,
    pub client_event_channels: Vec<(Option<ServerEventChannelTx>, Option<JoinHandle<()>>)>,
}
impl Session {
    pub fn start_game(&mut self, rq: StartGameEvent) -> anyhow::Result<Vec<proto::Card>> {
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
        let global_deck = self
            .game_state
            .as_ref()
            .unwrap()
            .card_context
            .cards
            .iter()
            .map(|e| proto::Card {
                player_id: e.player_id,
                number: e.number,
                color: e.color as i32,
                gender: e.gender as i32,
            })
            .collect();
        Ok(global_deck)
    }
}
impl TryFrom<proto::Play> for crate::Play {
    type Error = anyhow::Error;
    fn try_from(value: proto::Play) -> anyhow::Result<Self> {
        Ok(crate::Play {
            player: value.player_id,
            play: match value.play.unwrap() {
                play::Play::ArenaPlay(a) => crate::Action::Arena(match a.play_type() {
                    ArenaPlayType::FromAvailableHand => {
                        crate::ArenaAction::FromAvailableHand(a.from_index.unwrap())
                    }
                    ArenaPlayType::FromBlitz => {
                        crate::ArenaAction::FromBlitz(a.from_index.unwrap())
                    }
                    ArenaPlayType::FromPost => crate::ArenaAction::FromPost {
                        post_pile: a.from_index.unwrap(),
                        arena_pile: a.to_index.unwrap(),
                    },
                }),
                play::Play::PlayerPlay(p) => crate::Action::Player(match p.play_type() {
                    PlayerPlayType::BlitzToPost => {
                        crate::PlayerAction::BlitzToPost(p.post_index.unwrap())
                    }
                    PlayerPlayType::AvailableHandToPost => {
                        crate::PlayerAction::AvailableToPost(p.post_index.unwrap())
                    }
                    PlayerPlayType::TransferToAvailableHand => {
                        crate::PlayerAction::TransferToAvailable
                    }
                    PlayerPlayType::ResetHand => crate::PlayerAction::ResetHand,
                }),
                play::Play::CallBlitz(c) => crate::Action::CallBlitz(c.player_index),
            },
        })
    }
}

#[derive(Clone)]
pub struct Server {
    sessions: Arc<DashMap<String, Session>>,
}

impl Server {
    pub fn new() -> Self {
        Server {
            sessions: Arc::new(DashMap::new()),
        }
    }
    pub fn create_session(&self, rq: proto::StartSessionRq) -> tonic::Result<proto::Player> {
        let session_id = ObjectId::new().to_hex();
        //make sure the username is not blank
        if rq.username.is_empty() {
            return Err(tonic::Status::invalid_argument(
                "Username must not be blank",
            ));
        }
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
            client_event_channels: vec![(None, None)],
        };
        self.sessions.insert(session_id.clone(), session);
        info!(session_id, "Session created");
        Ok(player)
    }
    pub fn sv_join_session(&self, rq: JoinSessionRq) -> tonic::Result<Player> {
        let session_id = rq.session_id.clone();
        let mut session = self.sessions.get_mut(&session_id).ok_or_else(|| {
            tracing::error!(session_id, "No session found with provided id");
            tonic::Status::not_found("No session found with provided id")
        })?;
        if !session.is_joinable {
            return Err(tonic::Status::failed_precondition(
                "Session is not joinable!",
            ));
        }
        let player_game_id = session.players.len() as u32;
        let player = Player {
            session_id: session_id.clone(),
            player_game_id,
            username: rq.username,
            face_image_id: rq.face_image_id,
            is_session_admin: false,
        };
        //make sure no player with the given username exists
        if session
            .players
            .iter()
            .any(|p| p.username == player.username)
        {
            return Err(tonic::Status::invalid_argument(
                "Player with given username already exists",
            ));
        }
        session.players.push(player.clone());
        info!(
            session_id,
            player_id = player_game_id,
            "Player joined session"
        );
        session.client_event_channels.push((None, None));
        Ok(player)
    }

    async fn process_client_events(
        sessions: Arc<DashMap<String, Session>>,
        session_id: String,
        player_id: u32,
        cancel: tokio::sync::oneshot::Sender<()>,
        mut rx: impl Stream<Item = tonic::Result<ClientEvent>>
            + std::marker::Unpin
            + std::marker::Send
            + 'static,
    ) -> tokio::task::JoinHandle<()> {
        info!(session_id, player_id, "Client event processor working...");

        tokio::spawn(async move {
            loop {
                match rx.try_next().await {
                    Ok(Some(c)) => {
                        let e = c.event.unwrap();
                        let event_id = c.event_id;

                        match e {
                            client_event::Event::Play(p) => {
                                tracing::info!(
                                    player_id,
                                    event_id = c.event_id,
                                    "Play event received. Event: {p:?}"
                                );
                                if let Some(mut session) = sessions.get_mut(&session_id) {
                                    if let Some(g) = session.game_state.as_mut() {
                                        let event =
                                            g.make_play(p.try_into().unwrap()).into_tonic_status();
                                        if let Err(e) = &event {
                                            //send an error back to the player that sent this
                                            tracing::error!(
                                                session_id,
                                                player_id,
                                                "Could not play!"
                                            );
                                            let event =
                                                tonic::Status::unknown("Could not make play!");
                                            Self::send_event_to_client(
                                                Err(event),
                                                &session,
                                                player_id,
                                            )
                                            .await
                                            .with_context(|| {
                                                tracing::error!(
                                                    session_id,
                                                    player_id,
                                                    "Could not send event to client"
                                                );
                                                "Could not send event to client"
                                            })
                                            .unwrap();
                                        }
                                        //broadcast event to clients
                                        Self::broadcast_event(event, &session, player_id, event_id)
                                            .await
                                            .with_context(|| {
                                                tracing::error!(
                                                    "Could not send events to all clients"
                                                );
                                                "Could not send sevents to all events"
                                            })
                                            .unwrap();
                                    } else {
                                        tracing::error!(
                                            "Game not started. This should not be possible"
                                        );
                                        continue;
                                    }
                                } else {
                                    tracing::warn!(session_id, "Session does not exist");
                                    continue;
                                }
                            }
                            client_event::Event::ChangeDrawRate(c) => {
                                tracing::info!(
                                    player_id,
                                    event_id,
                                    "Change draw rate event received. Event: {c:?}"
                                );
                                if let Some(mut session) = sessions.get_mut(&session_id) {
                                    if let Some(g) = session.game_state.as_mut() {
                                        g.change_draw_rate(c.new_rate);
                                        let event = Ok(server_event::Event::ChangeDrawRate(
                                            ChangeDrawRateEvent {
                                                new_rate: c.new_rate,
                                            },
                                        ));

                                        //broadcast event to clients
                                        Self::broadcast_event(event, &session, player_id, event_id)
                                            .await
                                            .with_context(|| {
                                                tracing::error!(
                                                    "Could not send events to all clients"
                                                );
                                                "Could not send sevents to all events"
                                            })
                                            .unwrap();
                                    } else {
                                        tracing::error!(
                                            "Game not started. This should not be possible"
                                        );
                                        //send an error back to the player that sent this
                                        let event =
                                            tonic::Status::failed_precondition("Game not started!");
                                        Self::send_event_to_client(Err(event), &session, player_id)
                                            .await
                                            .with_context(|| {
                                                tracing::error!(
                                                    session_id,
                                                    player_id,
                                                    "Could not send event to client"
                                                );
                                                "Could not send event to client"
                                            })
                                            .unwrap();
                                        continue;
                                    }
                                } else {
                                    tracing::warn!(session_id, "Session does not exist");
                                    //send an error back to the player that sent this
                                    let event = tonic::Status::not_found("Session does not exist!");
                                    if let Some(session) = sessions.get_mut(&session_id) {
                                        Self::send_event_to_client(Err(event), &session, player_id)
                                            .await
                                            .with_context(|| {
                                                tracing::error!(
                                                    session_id,
                                                    player_id,
                                                    "Could not send event to client"
                                                );
                                                "Could not send event to client"
                                            })
                                            .unwrap();
                                    }
                                }
                            }
                            client_event::Event::StaticEvent(s) => {
                                let client_game_state_action: ClientGameStateAction =
                                    s.try_into().unwrap();
                                tracing::info!(
                                    player_id,
                                    event_id,
                                    "Static event received. Event: {client_game_state_action:?}"
                                );
                                if let Some(mut session) = sessions.get_mut(&session_id) {
                                    if player_id != 0 {
                                        tracing::warn!(
                                            "Only the session admin can call this event"
                                        );
                                        let e = Err(tonic::Status::failed_precondition(
                                            "Only the server admin can send these events!",
                                        ));
                                        Self::broadcast_event(e, &session, player_id, event_id)
                                            .await
                                            .unwrap();
                                    }
                                    let e = match client_game_state_action {
                                        ClientGameStateAction::PauseGame => {
                                            proto::server_event::Event::ServerGameStateAction(
                                                ServerGameStateAction::ServerPauseGame as i32,
                                            )
                                        }
                                        ClientGameStateAction::ResumeGame => {
                                            proto::server_event::Event::ServerGameStateAction(
                                                ServerGameStateAction::ServerResumeGame as i32,
                                            )
                                        }

                                        ClientGameStateAction::ResetDrawRate => {
                                            if let Some(g) = session.game_state.as_mut() {
                                                g.change_draw_rate(3)
                                            }
                                            proto::server_event::Event::ChangeDrawRate(
                                                ChangeDrawRateEvent { new_rate: 3 },
                                            )
                                        }
                                    };
                                    Self::broadcast_event(Ok(e), &session, player_id, event_id)
                                        .await
                                        .unwrap();
                                }
                            }

                            client_event::Event::OpenStream(_) => {
                                tracing::warn!("OpenStream event received. This should not happen");
                                //send an error back to the player that sent this
                                let event = tonic::Status::failed_precondition(
                                    "OpenStream event received when channel is already open!",
                                );
                                if let Some(session) = sessions.get(&session_id) {
                                    Self::send_event_to_client(Err(event), &session, player_id)
                                        .await
                                        .with_context(|| {
                                            tracing::error!(
                                                session_id,
                                                player_id,
                                                "Could not send event to client"
                                            );
                                            "Could not send event to client"
                                        })
                                        .unwrap();
                                }
                            }
                            client_event::Event::StartGame(s) => {
                                tracing::info!(
                                    player_id,
                                    event_id,
                                    "StartGame event received. Event: {s:?}"
                                );
                                info!(player_id = player_id, "StartGame event received");
                                if player_id != 0 {
                                    //the game can only be started by player 0 (admin)
                                    tracing::warn!("Player {} tried to start game", player_id);
                                    continue;
                                }
                                let mut session = sessions.get_mut(&session_id).unwrap();
                                let global_deck = session
                                    .start_game(s.clone())
                                    .with_context(|| "Failed to start game")
                                    .unwrap();
                                info!(session_id = session_id, "Game started");
                                let e = server_event::Event::StartGame(ServerStartGameEvent {
                                    prefs: s.prefs.clone(),
                                    global_deck: Some(proto::GlobalDeck { cards: global_deck }),
                                });
                                Self::broadcast_event(Ok(e), &session, player_id, event_id)
                                    .await
                                    .with_context(|| "Failed to broadcast event")
                                    .unwrap();
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error receiving event: {}", e);
                        break;
                    }
                    Ok(None) => {
                        tracing::error!("Client disconnected");
                        break;
                    }
                }
            }
            //cancel
            if let Err(_) = cancel.send(()) {
                tracing::info!("Failed to send cancel signal to client event processor");
            }
        })
    }
    pub async fn broadcast_event(
        event: tonic::Result<server_event::Event>,
        session: &Session,
        player_id: u32,
        event_id: u32,
    ) -> anyhow::Result<()> {
        info!(
            "Trying to broadcast event to {} players",
            session.players.len()
        );

        for player in session.players.iter() {
            if let (Some(tx), _) = session
                .client_event_channels
                .get(player_id as usize)
                .with_context(|| {
                    tracing::error!("Could not send event to client. Invalid player ");
                    "Could not send event to client. Invalid player "
                })?
            {
                if player.player_game_id == player_id {
                    //send acknoledgement to sender
                    info!(
                        session.id,
                        player_id = player_id,
                        "Sending acknowledgement to client"
                    );
                    let ack = Acknowledge { event_id };
                    tx.send(Ok(ServerEvent {
                        event: Some(server_event::Event::Acknowledge(ack)),
                    }))
                    .unwrap();
                    info!(player_id = player_id, "Sent acknowledgement")
                } else {
                    info!(
                        session.id,
                        player_id = player.player_game_id,
                        "Sending event to client"
                    );
                    tx.send(event.clone().map(|e| ServerEvent { event: Some(e) }))
                        .unwrap();
                    info!(player_id = player.player_game_id, "Sent event to client");
                }
            }
        }
        info!(session.id, player_id = player_id, "Event broadcasted");
        Ok(())
    }

    #[tracing::instrument(skip(session))]
    pub async fn send_event_to_client(
        event: tonic::Result<server_event::Event>,
        session: &Session,
        player_id: u32,
    ) -> anyhow::Result<()> {
        if let (Some(tx), _) = session
            .client_event_channels
            .get(player_id as usize)
            .with_context(|| {
                tracing::error!("Could not send event to client. Invalid player ");
                "Could not send event to client. Invalid player "
            })?
        {
            tx.send(event.clone().map(|e| ServerEvent { event: Some(e) }))
                .unwrap();
            info!(player_id = player_id, "Sent event to client");
        } else {
            tracing::error!(
                player_id,
                "Could not send event to client. Client not connected"
            );
        }
        Ok(())
    }
    pub fn sv_get_active_sessions(&self) -> proto::SessionRes {
        proto::SessionRes {
            sessions: self
                .sessions
                .iter()
                .map(|s| proto::Session {
                    id: s.id.clone(),
                    players: s.players.iter().map(|p| p.username.clone()).collect(),
                })
                .collect(),
        }
    }
    pub async fn sv_open_server_event_stream(
        &self,
        player: proto::Player,
    ) -> tonic::Result<EventChannelRx> {
        info!("Opening event stream");
        let (server_tx, server_rx) = flume::unbounded::<tonic::Result<ServerEvent>>();

        info!(session_id = player.session_id, "First message received");
        //send acknowledgement

        //add this channel to the sessions list of channels
        let mut session = self
            .sessions
            .get_mut(&player.session_id)
            .ok_or_else(|| tonic::Status::failed_precondition("No session found with id"))?;
        info!(session_id = player.session_id, "Session found");
        let id = player.player_game_id;
        let channel = session
            .client_event_channels
            .get_mut(id as usize)
            .with_context(|| format!("Player channel not found: {}", id))
            .into_tonic_status()?;
        if let (Some(_), _) = channel {
            tracing::error!("error: Channel already open");
            return Err(tonic::Status::failed_precondition("Channel already open"));
        } else {
            info!(
                session_id = player.session_id,
                "Channel opened. Starting event processing for client with player id: {}", id
            );

            session.client_event_channels[id as usize] = (Some(server_tx), None);
        }

        info!("Returning server channel");
        Ok(server_rx)
    }
    async fn open_client_event_stream(
        &self,
        mut rx: impl Stream<Item = tonic::Result<ClientEvent>> + Send + Unpin + 'static,
    ) -> tonic::Result<tokio::sync::oneshot::Receiver<()>> {
        //the first message must be the OpenStream event
        info!("Waiting for first message from client");
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
        match rx.try_next().await {
            Ok(Some(c)) => {
                let e = c.event.unwrap();
                match e {
                    client_event::Event::OpenStream(e) => {
                        tracing::info!("OpenStream event received");
                        let player = e.player.unwrap();
                        let mut session =
                            self.sessions.get_mut(&player.session_id).ok_or_else(|| {
                                tonic::Status::failed_precondition("No session found with id")
                            })?;
                        let id = player.player_game_id;
                        let channel = session
                            .client_event_channels
                            .get_mut(id as usize)
                            .with_context(|| format!("Player channel not found: {}", id))
                            .into_tonic_status()?;
                        if let (Some(_), None) = channel {
                            //send acknowledgement
                            tracing::info!(
                                session_id = player.session_id,
                                player_id = id,
                                "Sending acknowledgement to client"
                            );
                            let ack = Acknowledge { event_id: 0 };
                            let event = proto::server_event::Event::Acknowledge(ack);
                            Self::send_event_to_client(Ok(event), &session, id)
                                .await
                                .with_context(|| {
                                    tracing::error!(
                                        session_id = player.session_id,
                                        player_id = id,
                                        "Could not send event to client"
                                    );
                                    "Could not send event to client"
                                })
                                .unwrap();
                            //spawn the client event processor and store the join handle
                            let join_handle = Self::process_client_events(
                                self.sessions.clone(),
                                player.session_id.clone(),
                                id,
                                cancel_tx,
                                rx,
                            )
                            .await;
                            info!(
                                session_id = player.session_id,
                                player_id = id,
                                "Client event processor spawned"
                            );

                            session.client_event_channels[id as usize].1 = Some(join_handle);
                        }
                    }
                    _ => {
                        tracing::error!("First event must be OpenStream");
                        return Err(tonic::Status::failed_precondition(
                            "First event must be OpenStream",
                        ));
                    }
                }
            }
            Err(e) => {
                tracing::error!("Error receiving event: {}", e);
                return Err(tonic::Status::unknown("Error receiving event"));
            }
            Ok(None) => {
                tracing::error!("Client disconnected");
                return Err(tonic::Status::unknown("Client disconnected"));
            }
        }
        Ok(cancel_rx)
    }
    ///Ends the session that the player is in. Ends game for all players. Can only be called by the session admin
    pub async fn sv_end_session(&self, player: &proto::Player) -> tonic::Result<()> {
        info!(
            player_id = player.player_game_id,
            session_id = player.session_id,
            "Trying to end session"
        );
        let session_id = &player.session_id;
        let player_id = player.player_game_id;
        let mut session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| tonic::Status::not_found("No session found with provided id"))?;
        info!(session_id, player_id, "Session found");
        if player_id == 0 {
            //session admin
            if session.game_state.is_some() {
                //send game over event to all players
                let event = Ok(server_event::Event::ServerGameStateAction(
                    ServerGameStateAction::ServerGameOver as i32,
                ));
                info!(session_id, "Sending game over event to all clients");
                Self::broadcast_event(event, &session, player_id, 0)
                    .await
                    .with_context(|| {
                        tracing::error!("Could not send events to all clients");
                        "Could not send sevents to all events"
                    })
                    .into_tonic_status()?;
                info!(session_id, "Game over event sent to all clients");
                //end game
                session.game_state = None;
                //close all join handles
                for channel in session.client_event_channels.iter_mut() {
                    if let (_, Some(handle)) = channel {
                        handle.abort();
                    }
                }
                //delete session
                drop(session);
                self.sessions.remove(session_id);
                tracing::info!(session_id, "Session ended successfully");
            } else {
                tracing::warn!(session_id, "Session is not in game");
                return Err(tonic::Status::failed_precondition("Session is not in game"));
            }
        } else {
            //just remove the session from the list of sessions, since no game was started
            self.sessions.remove(session_id);
        }
        Ok(())
    }
}
#[tonic::async_trait]
impl proto::session_service_server::SessionService for Server {
    async fn get_active_sessions(
        &self,
        _: tonic::Request<()>,
    ) -> std::result::Result<tonic::Response<SessionRes>, tonic::Status> {
        let sessions = self.sv_get_active_sessions();
        Ok(tonic::Response::new(sessions))
    }
    /// Join an active session
    async fn join_session(
        &self,
        request: tonic::Request<JoinSessionRq>,
    ) -> std::result::Result<tonic::Response<Player>, tonic::Status> {
        let rq = request.into_inner();
        let player = self.sv_join_session(rq)?;
        Ok(tonic::Response::new(player))
    }
    /// Start a new session that other players can join. The player who starts the session is the session admin. It then returns a Player message
    /// with the player_id field set to the player's unique identifier
    async fn start_session(
        &self,
        request: tonic::Request<StartSessionRq>,
    ) -> std::result::Result<tonic::Response<Player>, tonic::Status> {
        let rq = request.into_inner();
        let player = self.create_session(rq)?;
        Ok(tonic::Response::new(player))
    }
    async fn end_session(
        &self,
        request: tonic::Request<proto::Player>,
    ) -> std::result::Result<tonic::Response<()>, tonic::Status> {
        let player = request.into_inner();
        self.sv_end_session(&player).await?;
        Ok(tonic::Response::new(()))
    }
    async fn get_session(
        &self,
        request: tonic::Request<GetSessionRq>,
    ) -> std::result::Result<tonic::Response<proto::Session>, tonic::Status> {
        let rq = request.into_inner();
        let session = self
            .sessions
            .get(&rq.session_id)
            .ok_or_else(|| tonic::Status::not_found("No session found with provided id"))?;
        let session = proto::Session {
            id: session.id.clone(),
            players: session.players.iter().map(|p| p.username.clone()).collect(),
        };
        Ok(tonic::Response::new(session))
    }
}
type ResponseStream =
    std::pin::Pin<Box<dyn Stream<Item = Result<ServerEvent, tonic::Status>> + Send>>;
#[tonic::async_trait]
impl proto::game_service_server::GameService for Server {
    type OpenEventStreamStream = ResponseStream;
    async fn open_event_stream(
        &self,
        request: tonic::Request<proto::Player>,
    ) -> std::result::Result<tonic::Response<Self::OpenEventStreamStream>, tonic::Status> {
        let client_id = request.local_addr().unwrap().to_string();
        let rq = request.into_inner();

        let rx = self.sv_open_server_event_stream(rq).await?;
        let stream = rx.into_stream();
        let stream = stream.map(move |e| {
            tracing::info!(
                client_addr = client_id,
                "Server stream recieved event: {:?}",
                e
            );
            e
        });
        Ok(tonic::Response::new(Box::pin(stream) as ResponseStream))
    }
    /// Open client event stream. This is used to send events to the server
    async fn open_client_event_stream(
        &self,
        request: tonic::Request<tonic::Streaming<proto::ClientEvent>>,
    ) -> std::result::Result<tonic::Response<()>, tonic::Status> {
        let mut stream = request.into_inner();
        let (stream_tx, stream_rx) = flume::unbounded();
        let proc_rx = tokio::spawn(async move {
            loop {
                match stream.try_next().await {
                    Ok(Some(v)) => {
                        stream_tx.send(Ok(v)).unwrap();
                    }
                    Ok(None) => {
                        tracing::error!("Client disconnected");
                        break;
                    }
                    Err(e) => {
                        tracing::error!("Error receiving event: {}", e);
                        break;
                    }
                }
            }
        });
        let cancel = self
            .open_client_event_stream(stream_rx.into_stream())
            .await?;
        tokio::select! {
            _ = proc_rx => {
                tracing::info!("Client event processor ended");
            }
            _ = cancel => {
                tracing::info!("Client event processor cancelled");
            }
        }
        Ok(tonic::Response::new(()))
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
