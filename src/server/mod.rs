use std::fmt::Display;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
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
use crate::server;
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
    pub server_event_counter: AtomicU32,
    //Map of events that are in flight for each client. The key is the player id
    pub in_flight_events: Arc<DashMap<u32, Vec<u32>>>,
    pub id: String,
    ///whether or not this session can be joined by others. When not true,
    ///  this game is either already being  or about to be played
    pub is_joinable: bool,
    pub game_state: Option<GameState>,
    pub players: Vec<Player>,
    pub client_event_channels: Vec<(
        Option<ServerEventChannelTx>,
        Option<JoinHandle<core::result::Result<(), anyhow::Error>>>,
    )>,
}
impl Session {
    pub fn next_event_id(&self) -> u32 {
        self.server_event_counter.fetch_add(1, Ordering::Relaxed)
    }
    pub fn start_game(
        &mut self,
        rq: StartGameEvent,
    ) -> anyhow::Result<(Vec<proto::Card>, Vec<proto::PlayerCards>)> {
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
        let player_cards =
            self.game_state
                .as_ref()
                .unwrap()
                .players
                .iter()
                .map(|p| proto::PlayerCards {
                    hand: Some(PlayerHand {
                        in_hand: p.hand.in_hand.clone(),
                        available_to_play: p.hand.available_to_play.clone(),
                    }),
                    post: Some(PostPile {
                        piles: p
                            .post_pile
                            .piles
                            .iter()
                            .map(|e| proto::Pile {
                                cards: e.cards.clone(),
                            })
                            .collect(),
                    }),
                    blitz_pile: p.blitz_pile.cards.clone(),
                });
        Ok((global_deck, player_cards.collect()))
    }

    pub async fn sv_close_channel(&mut self, player_id: u32) -> tonic::Result<()> {
        info!(player_id = player_id, "Trying to end session");
        //make sure the player is in the session
        if player_id >= self.players.len() as u32 {
            return Err(tonic::Status::not_found("Player not found in session"));
        }
        let session_id = &self.id;
        let Some(player)=self.players.get_mut(player_id as usize)else {
            return Err(tonic::Status::not_found("Player not found in session"));

        };
        let is_admin = player.is_session_admin;

        //session admin
        if self.game_state.is_some() {
            //send game over event to all players
            let event = Ok(server_event::Event::ServerGameStateAction(
                ServerGameStateAction::ServerGameOver as i32,
            ));
            info!(session_id, "Sending game over event to all clients");
            Server::broadcast_event(event, self, player_id, 0, false)
                .await
                .with_context(|| {
                    tracing::error!("Could not send events to all clients");
                    "Could not send sevents to all events"
                })
                .into_tonic_status()?;
            info!(session_id, "Game over event sent to all clients");
            //end game
            self.game_state = None;
            //close all join handles
            for channel in self.client_event_channels.iter_mut() {
                if let (_, Some(handle)) = channel {
                    handle.abort();
                }
            }

            tracing::info!(session_id, "Session ended successfully");
        } else {
            //just close the channel
            if let Some((_, Some(handle))) = self.client_event_channels.get_mut(player_id as usize)
            {
                handle.abort();
            }
            //delete channel
            self.client_event_channels.remove(player_id as usize);
            //reorganize player ids
            //set the next player to be the admin
            if is_admin {
                if let Some(p) = self.players.get_mut(player_id as usize + 1) {
                    p.is_session_admin = true;
                }
            }

            //remove the player
            self.players.remove(player_id as usize);
        }

        Ok(())
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
                        crate::ArenaAction::FromAvailableHand(a.to_index.unwrap())
                    }
                    ArenaPlayType::FromBlitz => {
                        crate::ArenaAction::FromBlitz(a.to_index.unwrap())
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
            in_flight_events: Arc::new(DashMap::new()),
            server_event_counter: AtomicU32::new(0),
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

    async fn send_ack_event(
        session: &Session,
        player_id: u32,
        event_id: u32,
        ack_type: EAcknowledgementType,
        message: String,
    ) -> anyhow::Result<()> {
        let ack = Acknowledge {
            event_id,
            acknowledgement_type: ack_type as i32,
            message,
        };
        let event = server_event::Event::Acknowledge(ack);
        Self::send_event_to_client(Ok(event), session, player_id, event_id)
            .await
            .with_context(|| "Failed to send event to client")?;
        Ok(())
    }
    async fn process_client_events(
        sessions: Arc<DashMap<String, Session>>,
        session_id: String,
        player_id: u32,
        cancel: tokio::sync::oneshot::Sender<anyhow::Result<()>>,
        mut rx: impl Stream<Item = tonic::Result<ClientEvent>>
            + std::marker::Unpin
            + std::marker::Send
            + 'static,
    ) -> tokio::task::JoinHandle<anyhow::Result<()>> {
        info!(session_id, player_id, "Client event processor working...");

        tokio::spawn(async move {
            let res = || async {
                loop {
                    match rx.try_next().await {
                        Ok(Some(c)) => {
                            let e = c.event.unwrap();
                            let client_event_id = c.event_id;

                            match e {
                                client_event::Event::Play(p) => {
                                    tracing::info!(
                                        player_id,
                                        event_id = c.event_id,
                                        "Play event received. Event: {p:?}"
                                    );
                                    let Some(mut session) = sessions.get_mut(&session_id) else{
                                        tracing::warn!(session_id, "Session does not exist");
                                        continue
                                    };
                                    let Some(g) = session.game_state.as_mut() else{
                                        tracing::error!("Game not started. This should not be possible");
                                        continue
                                    };

                                    let event = g.make_play(p.try_into().unwrap());
                                    match &event {
                                        Err(e) => {
                                            //send an error back to the player that sent this
                                            tracing::warn!(
                                                session_id,
                                                player_id,
                                                "Could not play!"
                                            );
                                            let event =
                                                server_event::Event::GamePlayError(GamePlayError {
                                                    message: e.to_string(),
                                                });
                                            Self::send_event_to_client(
                                                Ok(event),
                                                &session,
                                                player_id,
                                                session.next_event_id(),
                                            )
                                            .await
                                            .with_context(|| {
                                                tracing::error!(
                                                    session_id,
                                                    player_id,
                                                    "Could not send event to client"
                                                );
                                                "Could not send event to client"
                                            })?;
                                            Self::send_ack_event(&session, player_id, session.next_event_id(), EAcknowledgementType::Rejected, format!("{e:?}")).await?;
                                        }
                                        Ok(e) => {
                                            Self::broadcast_event(
                                                Ok(e.clone()),
                                                &session,
                                                player_id,
                                                client_event_id,
                                                true,
                                            )
                                            .await
                                            .with_context(|| {
                                                tracing::error!(
                                                    "Could not send events to all clients"
                                                );
                                                "Could not send sevents to all events"
                                            })?;
                                        }
                                    }
                                }
                                client_event::Event::ChangeDrawRate(c) => {
                                    tracing::info!(
                                        player_id,
                                        client_event_id,
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
                                            Self::broadcast_event(
                                                event,
                                                &session,
                                                player_id,
                                                client_event_id,
                                                false,
                                            )
                                            .await
                                            .with_context(|| {
                                                tracing::error!(
                                                    "Could not send events to all clients"
                                                );
                                                "Could not send sevents to all events"
                                            })?
                                        } else {
                                            tracing::error!(
                                                "Game not started. This should not be possible"
                                            );
                                            //send an error back to the player that sent this
                                            let event = tonic::Status::failed_precondition(
                                                "Game not started!",
                                            );
                                            Self::send_event_to_client(
                                                Err(event),
                                                &session,
                                                player_id,
                                                session.next_event_id(),
                                            )
                                            .await
                                            .with_context(|| {
                                                tracing::error!(
                                                    session_id,
                                                    player_id,
                                                    "Could not send event to client"
                                                );
                                                "Could not send event to client"
                                            })?;
                                            continue;
                                        }
                                    } else {
                                        tracing::warn!(session_id, "Session does not exist");
                                        //send an error back to the player that sent this
                                        let event =
                                            tonic::Status::not_found("Session does not exist!");
                                        if let Some(session) = sessions.get_mut(&session_id) {
                                            Self::send_event_to_client(
                                                Err(event),
                                                &session,
                                                session.next_event_id(),
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
                                            })?
                                        }
                                    }
                                }
                                client_event::Event::StaticEvent(s) => {
                                    let client_game_state_action: ClientGameStateAction =
                                        s.try_into().unwrap();
                                    tracing::info!(
                                    player_id,
                                    client_event_id,
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
                                            Self::broadcast_event(
                                                e,
                                                &session,
                                                player_id,
                                                client_event_id,
                                                false,
                                            )
                                            .await?
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
                                        Self::broadcast_event(
                                            Ok(e),
                                            &session,
                                            player_id,
                                            client_event_id,
                                            false,
                                        )
                                        .await?
                                    }
                                }

                                client_event::Event::OpenStream(_) => {
                                    tracing::warn!(
                                        "OpenStream event received. This should not happen"
                                    );
                                    //send an error back to the player that sent this
                                    let event = tonic::Status::failed_precondition(
                                        "OpenStream event received when channel is already open!",
                                    );
                                    if let Some(session) = sessions.get(&session_id) {
                                        Self::send_event_to_client(
                                            Err(event),
                                            &session,
                                            player_id,
                                            session.next_event_id(),
                                        )
                                        .await
                                        .with_context(
                                            || {
                                                tracing::error!(
                                                    session_id,
                                                    player_id,
                                                    "Could not send event to client"
                                                );
                                                "Could not send event to client"
                                            },
                                        )?
                                    }
                                }
                                client_event::Event::StartGame(s) => {
                                    tracing::info!(
                                        player_id,
                                        client_event_id,
                                        "StartGame event received. Event: {s:?}"
                                    );
                                    info!(player_id = player_id, "StartGame event received");
                                    if player_id != 0 {
                                        //the game can only be started by player 0 (admin)
                                        tracing::warn!("Player {} tried to start game", player_id);
                                        continue;
                                    }
                                    let mut session = sessions.get_mut(&session_id).unwrap();
                                    let (global_deck, player_cards) = session
                                        .start_game(s.clone())
                                        .with_context(|| "Failed to start game")?;
                                    info!(session_id = session_id, "Game started");

                                    tracing::debug!("Player cards {player_cards:?}");
                                    let server_event_id = session.next_event_id();
                                    for pid in 1..player_cards.len() {
                                        let e = server_event::Event::RequestStartGame(
                                            ServerRequestStartGameEvent {
                                                prefs: s.prefs.clone(),
                                                global_deck: Some(proto::GlobalDeck {
                                                    cards: global_deck.clone(),
                                                }),
                                                player_cards: player_cards.clone(),
                                            },
                                        );
                                        Self::send_event_to_client(
                                            Ok(e),
                                            &session,
                                            pid as u32,
                                            server_event_id,
                                        )
                                        .await
                                        .with_context(|| "Failed to send event to player")?;
                                    }

                                    let in_flight = session.in_flight_events.clone();
                                    drop(session);
                                    //wait for all players to ack
                                    info!("Waiting for all players to ack");
                                    loop {
                                        tracing::debug!("{in_flight:?}");
                                        let mut all_acked = true;
                                        info!("Checking if event has been acked");
                                        let in_flight_iter = in_flight.iter();
                                        for player in in_flight_iter {
                                            //check if this event_id has been removed from the in_flight list
                                            if player.value().contains(&server_event_id) {
                                                all_acked = false;
                                                break;
                                            }
                                        }
                                        if all_acked {
                                            info!("All players acked");
                                            break;
                                        }

                                        tokio::time::sleep(std::time::Duration::from_millis(100))
                                            .await;
                                    }
                                    //Send confirm game started to client
                                    info!("Sending confirm game started to client");
                                    let e = server_event::Event::ConfirmGameStart(
                                        ServerRequestStartGameEvent {
                                            prefs: s.prefs.clone(),
                                            global_deck: Some(proto::GlobalDeck {
                                                cards: global_deck,
                                            }),
                                            player_cards,
                                        },
                                    );
                                    let session = sessions.get(&session_id).unwrap();

                                    Self::send_event_to_client(
                                        Ok(e),
                                        &session,
                                        player_id,
                                        session.next_event_id(),
                                    )
                                    .await
                                    .with_context(|| "Failed to send event to client")?;
                                }
                                client_event::Event::Acknowledge(a) => {
                                    tracing::info!(
                                        player_id,
                                        client_event_id,
                                        "Acknowledge event received. Event: {a:?}"
                                    );
                                    if let Some(session) = sessions.get(&session_id) {
                                        if let Some(mut in_flight_events) =
                                            session.in_flight_events.get_mut(&player_id)
                                        {
                                            in_flight_events.retain(|e| *e != a.event_id);
                                            info!(
                                                player_id,
                                                client_event_id,
                                                "Removed event from in_flight list"
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Error receiving event: {}", e);
                            //cancel this session
                            if let Some(mut session) = sessions.get_mut(&session_id) {
                                let res = session.sv_close_channel(player_id).await;
                                if let Err(e) = res {
                                    tracing::error!("Could not end session: {}", e);
                                }
                            }
                            //trying to send cancel signal
                            if cancel.send(Err(e.into())).is_err() {
                                tracing::error!(
                                    "Error sending cancel signal. The reciever has already dropped"
                                );
                            }
                            break;
                        }
                        Ok(None) => {
                            tracing::error!("Client disconnected");
                            cancel.send(Ok(())).unwrap();
                            break;
                        }
                    }
                }
                Ok(())
            };
            if let Err(e) = res().await {
                tracing::error!("Error processing client events: {}", e);
                Err(e)
            } else {
                anyhow::Ok(())
            }
        })
    }
    pub async fn broadcast_event(
        event: tonic::Result<server_event::Event>,
        session: &Session,
        player_id: u32,
        server_event_id: u32,
        send_to_all: bool,
    ) -> anyhow::Result<()> {
        info!(
            "Trying to broadcast event to {} players",
            session.players.len()
        );
        for player in session.players.iter() {
            if let (Some(tx), _) = session
                .client_event_channels
                .get(player.player_game_id as usize)
                .with_context(|| {
                    tracing::error!("Could not send event to client. Invalid player ");
                    "Could not send event to client. Invalid player "
                })?
            {
                if player.player_game_id == player_id {
                    //send acknowledgement to sender
                    info!(
                        session.id,
                        player_id = player_id,
                        "Sending acknowledgement to client"
                    );
                    let ack = Acknowledge {
                        event_id: server_event_id,
                        acknowledgement_type: EAcknowledgementType::Accepted as i32,
                        message: "".to_string(),
                    };
                    if let Err(e) = tx.send(Ok(ServerEvent {
                        event_id: server_event_id,
                        event: Some(server_event::Event::Acknowledge(ack)),
                    })) {
                        tracing::error!("Could not send ack. Probably channel closed: {}", e);
                        continue;
                    }
                    info!(player_id = player_id, "Sent acknowledgement");
                    if !send_to_all {
                        continue;
                    }
                }

                {
                    info!(
                        session.id,
                        player_id = player.player_game_id,
                        "Sending event to client"
                    );
                    if let Err(e) = tx.send(event.clone().map(|e| ServerEvent {
                        event_id: server_event_id,
                        event: Some(e),
                    })) {
                        tracing::error!("Could not send event. Probably channel closed: {}", e);
                        continue;
                    }
                    info!(player_id = player.player_game_id, "Sent event to client");
                    //if not ack event, add to in_flight list
                    if let Ok(e) = &event {
                        match e {
                            server_event::Event::Acknowledge(_) => {}
                            _ => {
                                info!(
                                    player_id = player.player_game_id,
                                    "Adding event to in_flight list"
                                );
                                session
                                    .in_flight_events
                                    .entry(player.player_game_id)
                                    .or_insert_with(Vec::new)
                                    .push(server_event_id);
                            }
                        }
                    }
                }
            }
        }
        info!(session.id, player_id = player_id, "Event broadcasted");
        Ok(())
    }

    #[tracing::instrument(skip(session, event))]
    pub async fn send_event_to_client(
        event: tonic::Result<server_event::Event>,
        session: &Session,
        player_id: u32,
        event_id: u32,
    ) -> anyhow::Result<()> {
        if let Ok(e) = &event {
            info!("Event: {e}")
        }
        if let (Some(tx), _) = session
            .client_event_channels
            .get(player_id as usize)
            .with_context(|| {
                tracing::error!("Could not send event to client. Invalid player ");
                "Could not send event to client. Invalid player "
            })?
        {
            tx.send(event.clone().map(|e| ServerEvent {
                event_id,
                event: Some(e),
            }))
            .unwrap();
            info!(player_id = player_id, "Sent event to client");
        } else {
            tracing::error!(
                player_id,
                "Could not send event to client. Client not connected"
            );
        } //if not ack event, add to in_flight list
        if let Ok(e) = &event {
            match e {
                server_event::Event::Acknowledge(_) => {}
                _ => {
                    info!(player_id = player_id, "Adding event to in_flight list");
                    session
                        .in_flight_events
                        .entry(player_id)
                        .or_insert_with(Vec::new)
                        .push(event_id);
                }
            }
        }
        Ok(())
    }
    pub fn sv_get_active_sessions(&self) -> proto::SessionRes {
        proto::SessionRes {
            sessions: self
                .sessions
                .iter()
                .filter(|e| e.is_joinable)
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
    ) -> tonic::Result<(tokio::sync::oneshot::Sender<()>, EventChannelRx)> {
        info!("Opening event stream");
        let (server_tx, server_rx) = flume::unbounded::<tonic::Result<ServerEvent>>();
        let (drop_tx, drop_rx) = tokio::sync::oneshot::channel::<()>();
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
        let ses_arc = self.sessions.clone();
        tokio::spawn(async move {
            let _ = drop_rx.await;
            info!(
                session_id = player.session_id,
                "Drop signal received. Closing channel"
            );
            if let Some(mut session) = ses_arc.get_mut(&player.session_id) {
                let res = session.sv_close_channel(player.player_game_id).await;
                if let Err(e) = res {
                    tracing::error!("Could not end session: {}", e);
                }
            }
        });

        info!("Returning server channel");
        Ok((drop_tx, server_rx))
    }
    async fn open_client_event_stream(
        &self,
        mut rx: impl Stream<Item = tonic::Result<ClientEvent>> + Send + Unpin + 'static,
    ) -> tonic::Result<()> {
        //the first message must be the OpenStream event
        info!("Waiting for first message from client");
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<anyhow::Result<()>>();
        let mut player_option = None;
        match rx.try_next().await {
            Ok(Some(c)) => {
                let e = c.event.unwrap();
                match e {
                    client_event::Event::OpenStream(e) => {
                        tracing::info!("OpenStream event received");
                        let player = e.player.unwrap();
                        player_option = Some(player.clone());

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
                            let ack = Acknowledge {
                                event_id: 0,
                                acknowledgement_type: EAcknowledgementType::Accepted as i32,
                                message: "".to_string(),
                            };
                            let event = proto::server_event::Event::Acknowledge(ack);

                            Self::send_event_to_client(
                                Ok(event),
                                &session,
                                id,
                                session.next_event_id(),
                            )
                            .await
                            .with_context(|| {
                                tracing::error!(
                                    session_id = player.session_id,
                                    player_id = id,
                                    "Could not send event to client"
                                );
                                "Could not send event to client"
                            })
                            .map_err(|e| tonic::Status::unknown(e.to_string()))?;

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

        cancel_rx
            .await
            .map_err(|e| tonic::Status::unknown(e.to_string()))?;
        Ok(())
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
                Self::broadcast_event(event, &session, player_id, 0, false)
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
pub struct ServerStream<'a> {
    stream: flume::r#async::RecvStream<'a, tonic::Result<ServerEvent>>,
    //Called when the stream is dropped
    drop_signal: Option<tokio::sync::oneshot::Sender<()>>,
}

impl Stream for ServerStream<'_> {
    type Item = Result<ServerEvent, tonic::Status>;
    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        std::pin::Pin::new(&mut self.stream).poll_next(cx)
    }
}
impl Drop for ServerStream<'_> {
    fn drop(&mut self) {
        tracing::debug!("Server stream dropped");
        self.drop_signal.take().unwrap().send(()).unwrap();
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
        let player_id = rq.player_game_id;
        let (cancel, stream) = self.sv_open_server_event_stream(rq).await?;
        let stream = ServerStream {
            stream: stream.into_stream(),
            drop_signal: Some(cancel),
        };
        let stream = stream.map(move |e| {
            tracing::debug!(
                client_addr = client_id,
                player_id = player_id,
                "Server about to send event: {:?}",
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
        let stream = request.into_inner();

        self.open_client_event_stream(stream).await?;
        tracing::warn!("Client event stream ended");

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
impl Display for server_event::Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            server_event::Event::GameStateChange(_) => f.write_str("GameStateChange"),
            server_event::Event::Acknowledge(_) => f.write_str("Acknowledge"),
            server_event::Event::ServerGameStateAction(_) => f.write_str("ServerGameStateAction"),
            server_event::Event::RequestStartGame(_) => f.write_str("RequestStartGame"),
            server_event::Event::ChangeDrawRate(_) => f.write_str("ChangeDrawRate"),
            server_event::Event::ConfirmGameStart(_) => f.write_str("ConfirmGameStart"),
            server_event::Event::GamePlayError(_) => f.write_str("GamePlayError"),
        }
    }
}
