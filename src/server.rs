use std::str::FromStr;

use bson::oid::ObjectId;
use dashmap::DashMap;

use crate::{
    blitz::{self, ArenaPlay, EPlay, Play},
    proto::{self, ArenaPlayType, Player, SessionRes},
};
use anyhow::{anyhow, Context, Result};

///A session that is either currently waiting to be joined or is already being played
pub struct Session {
    pub id: ObjectId,
    ///whether or not this session can be joined by others. When not true, this game is either already being played or about to be
    pub is_joinable: bool,
    pub players: Vec<PlayerConnection>,
    pub game_state: Option<blitz::GameState>,
}
impl Session {
    ///Informs all other players in the session that a player has made a play and they should update their game state
    async fn notify_players_of_play(
        &mut self,
        play: &proto::Play,
        _player: &proto::Player,
    ) -> Result<()> {
        for player in self.players.iter_mut() {
            if player.player.id != player.player.id {
                player
                    .connection
                    .make_play(tonic::Request::new(play.clone()))
                    .await?;
            }
        }
        Ok(())
    }
    async fn notify_players_pause_game(&mut self, _player: &proto::Player) -> Result<()> {
        for player in self.players.iter_mut() {
            if player.player.id != player.player.id {
                player
                    .connection
                    .pause_game(tonic::Request::new(()))
                    .await?;
            }
        }
        Ok(())
    }
    async fn notify_players_resume_game(&mut self, _player: &proto::Player) -> Result<()> {
        for player in self.players.iter_mut() {
            if player.player.id != player.player.id {
                player
                    .connection
                    .resume_game(tonic::Request::new(()))
                    .await?;
            }
        }
        Ok(())
    }
    async fn notify_players_change_draw_rate(
        &mut self,
        req: &proto::ChangeDrawRateRq,
    ) -> Result<()> {
        for player in self.players.iter_mut() {
            if player.player.id != req.player.as_ref().unwrap().id {
                player.connection.change_draw_rate(req.clone()).await?;
            }
        }
        Ok(())
    }
    async fn notify_players_reset_draw_rate(&mut self, req: &proto::ResetDrawRateRq) -> Result<()> {
        for player in self.players.iter_mut() {
            if player.player.id != req.player.as_ref().unwrap().id {
                player.connection.reset_draw_rate(req.clone()).await?;
            }
        }
        Ok(())
    }
}
pub struct PlayerConnection {
    pub player: Player,
    connection: proto::player_service_client::PlayerServiceClient<tonic::transport::Channel>,
}
//impl hash for session by just using the id
impl std::hash::Hash for Session {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
impl std::cmp::PartialEq for Session {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub struct SessionMan {
    pub sessions: DashMap<ObjectId, Session>,
}

#[tonic::async_trait]
impl proto::session_service_server::SessionService for SessionMan {
    async fn get_active_sessions(
        &self,
        _request: tonic::Request<()>,
    ) -> Result<tonic::Response<proto::SessionRes>, tonic::Status> {
        let sessions = self.sessions.iter().map(|r| r);
        let available_sessions = sessions
            .filter(|s| s.is_joinable)
            .map(|s| proto::Session {
                id: s.id.to_hex(),
                players: s.players.iter().map(|p| p.player.id.clone()).collect(),
            })
            .collect();

        let ses_res = SessionRes {
            sessions: available_sessions,
        };
        Ok(tonic::Response::new(ses_res))
    }
    async fn join_session(
        &self,
        request: tonic::Request<proto::JoinSessionRq>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        let remote_addr = request.remote_addr().unwrap();
        //create a grpc channel to the player
        let player_client = proto::player_service_client::PlayerServiceClient::connect(format!(
            "http://{}",
            remote_addr
        ))
        .await
        .unwrap();

        let rq = request.into_inner();
        let mut player = rq.player.unwrap();

        let session_id = bson::oid::ObjectId::from_str(rq.session_id.as_ref()).unwrap();
        let mut session = self
            .sessions
            .get_mut(&session_id)
            .ok_or(tonic::Status::not_found("session not found"))?;

        if !session.is_joinable {
            return Err(tonic::Status::failed_precondition(
                "session is not joinable",
            ));
        }
        player.player_face_id = (session.players.len() - 1) as u32;

        let player_con = PlayerConnection {
            player: player,
            connection: player_client,
        };
        session.players.push(player_con);
        Ok(tonic::Response::new(()))
    }
    async fn start_session(
        &self,
        request: tonic::Request<proto::StartSessionRq>,
    ) -> Result<tonic::Response<Player>, tonic::Status> {
        let addr = request.remote_addr().unwrap();
        let player_client =
            proto::player_service_client::PlayerServiceClient::connect(format!("http://{}", addr));
        let rq = request.into_inner();

        let session_id = ObjectId::new();
        let player_id = ObjectId::new();
        let player = proto::Player {
            id: player_id.to_hex(),
            session_id: session_id.to_hex(),
            player_name: rq.playername,
            player_face_id: 0,
            is_session_admin: true,
            face_image_id: rq.face_image_id,
        };
        let session = Session {
            id: session_id,
            is_joinable: true,
            players: vec![PlayerConnection {
                player: player.clone(),
                connection: player_client.await.unwrap(),
            }],
            game_state: None,
        };
        self.sessions.insert(session_id, session);
        Ok(tonic::Response::new(player))
    }
    async fn start_game(
        &self,
        request: tonic::Request<proto::StartGameRq>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        let rq = request.into_inner();
        let session_id =
            ObjectId::from_str(rq.player.as_ref().unwrap().session_id.as_ref()).unwrap();
        let mut session = self.sessions.get_mut(&session_id).unwrap();
        //only a session admin can start the game
        if !session
            .players
            .iter()
            .any(|p| p.player.id == rq.player.as_ref().unwrap().id && p.player.is_session_admin)
        {
            return Err(tonic::Status::permission_denied(
                "only session admin can start game",
            ));
        }
        let prefs = rq.prefs.as_ref().unwrap();
        let draw_rate = prefs.draw_rate as u8;
        let post_pile_size = prefs.post_pile_size as u8;
        let score_to_win = prefs.score_to_win as u16;
        let blitz_deduction = prefs.blitz_deduction as u16;
        let player_count = session.players.len() as u8;
        //start the game
        let game_state = blitz::GameState::new(
            draw_rate,
            post_pile_size,
            player_count,
            score_to_win,
            blitz_deduction,
        )
        .unwrap();
        //return error if the game is already started
        if session.game_state.is_some() {
            return Err(tonic::Status::failed_precondition("game already started"));
        }
        session.game_state = Some(game_state);
        Ok(tonic::Response::new(()))
    }
}
impl TryFrom<proto::Play> for Play {
    type Error = anyhow::Error;

    fn try_from(value: proto::Play) -> Result<Self, Self::Error> {
        let eplay = match value.play.ok_or_else(|| anyhow!("No play defined!"))? {
            proto::play::Play::ArenaPlay(p) => {
                let arena_play_vals = (p.from_index, p.to_index);
                let arena_play = match ArenaPlayType::from_i32(p.play_type).unwrap() {
                    ArenaPlayType::FromAvailableHand => {
                        ArenaPlay::FromAvailableHand(arena_play_vals.0.unwrap() as u8)
                    }
                    ArenaPlayType::FromBlitz => {
                        ArenaPlay::FromBlitz(arena_play_vals.0.unwrap() as u8)
                    }
                    ArenaPlayType::FromPost => ArenaPlay::FromPost((
                        arena_play_vals.0.unwrap() as u8,
                        arena_play_vals.1.unwrap() as u8,
                    )),
                };
                EPlay::Arena(arena_play)
            }
            proto::play::Play::PlayerPlay(p) => {
                let p_play_val = p.post_index.unwrap() as u8;
                let player_play = match proto::PlayerPlayType::from_i32(p.play_type).unwrap() {
                    proto::PlayerPlayType::BlitzToPost => {
                        blitz::PlayerPlay::BlitzToPost(p_play_val)
                    }
                    proto::PlayerPlayType::AvailableHandToPost => {
                        blitz::PlayerPlay::AvailableToPost(p_play_val)
                    }
                    proto::PlayerPlayType::TransferToAvailableHand => {
                        blitz::PlayerPlay::TransferToAvailable
                    }
                    proto::PlayerPlayType::ResetHand => blitz::PlayerPlay::ResetHand,
                };
                EPlay::PlayerPlay(player_play)
            }
            proto::play::Play::CallBlitz(p) => {
                let blitz_val = p.player_index as u8;
                blitz::EPlay::CallBlitz(blitz_val)
            }
        };
        let player_id = value.player.unwrap().player_face_id as u8;
        Ok(blitz::Play {
            player: player_id,
            play: eplay,
        })
    }
}

#[tonic::async_trait]
impl proto::game_service_server::GameService for SessionMan {
    async fn make_play(
        &self,
        request: tonic::Request<proto::Play>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        let proto_play = request.into_inner();
        let ses_id = ObjectId::from_str(&proto_play.player.as_ref().unwrap().session_id)
            .with_context(|| {
                format!(
                    "invalid session id: {}",
                    proto_play.player.as_ref().unwrap().session_id
                )
            });
        //convert into tonic::Status message
        let ses_id = ses_id.map_err(|e| tonic::Status::invalid_argument(e.to_string()))?;
        let play = Play::try_from(proto_play.clone()).unwrap();
        let mut session = self.sessions.get_mut(&ses_id).unwrap();
        let game_state = session.game_state.as_mut().unwrap();
        game_state.make_play(play).unwrap();
        let res = session
            .notify_players_of_play(&proto_play, proto_play.player.as_ref().unwrap())
            .await
            .with_context(|| format!("failed to notify players of play: {:?}", proto_play));

        //convert into tonic::Status message
        res.map_err(|e| tonic::Status::internal(e.to_string()))?;
        Ok(tonic::Response::new(()))
    }
    ///Change draw rate. Only session admin can do this
    async fn change_draw_rate(
        &self,
        request: tonic::Request<proto::ChangeDrawRateRq>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        let rq = request.into_inner();
        let ses_id =
            ObjectId::from_str(&rq.player.as_ref().unwrap().session_id).with_context(|| {
                format!(
                    "invalid session id: {}",
                    rq.player.as_ref().unwrap().session_id
                )
            });
        //convert into tonic::Status message
        let ses_id = ses_id.map_err(|e| tonic::Status::invalid_argument(e.to_string()))?;
        let mut session = self.sessions.get_mut(&ses_id).unwrap();
        let game_state = session.game_state.as_mut().unwrap();
        //the player must be a session admin
        if !rq.player.as_ref().unwrap().is_session_admin {
            return Err(tonic::Status::permission_denied(
                "only session admin can change draw rate",
            ));
        }
        game_state.change_draw_rate(rq.draw_rate as u8);
        //notify all players of the change
        let res = session.notify_players_change_draw_rate(&rq).await;
        //convert into tonic::Status message
        res.map_err(|e| tonic::Status::internal(e.to_string()))?;
        Ok(tonic::Response::new(()))
    }
    ///Reset draw rate to default. Only session admin can do this
    async fn reset_draw_rate(
        &self,
        request: tonic::Request<proto::ResetDrawRateRq>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        let rq = request.into_inner();
        let ses_id =
            ObjectId::from_str(&rq.player.as_ref().unwrap().session_id).with_context(|| {
                format!(
                    "invalid session id: {}",
                    rq.player.as_ref().unwrap().session_id
                )
            });
        //convert into tonic::Status message
        let ses_id = ses_id.map_err(|e| tonic::Status::invalid_argument(e.to_string()))?;
        let mut session = self.sessions.get_mut(&ses_id).unwrap();
        let game_state = session.game_state.as_mut().unwrap();
        //the player must be a session admin
        if !rq.player.as_ref().unwrap().is_session_admin {
            return Err(tonic::Status::permission_denied(
                "only session admin can reset draw rate",
            ));
        }
        game_state.reset_draw_rate();
        //notify all players of the change
        let res = session.notify_players_reset_draw_rate(&rq).await;
        //convert into tonic::Status message
        res.map_err(|e| tonic::Status::internal(e.to_string()))?;
        Ok(tonic::Response::new(()))
    }

    async fn pause_game(
        &self,
        request: tonic::Request<proto::Player>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        let player = request.into_inner();
        let ses_id = ObjectId::from_str(&player.session_id)
            .with_context(|| format!("invalid session id: {}", player.session_id));
        //convert into tonic::Status message
        let ses_id = ses_id.map_err(|e| tonic::Status::invalid_argument(e.to_string()))?;
        let mut session = self.sessions.get_mut(&ses_id).unwrap();

        //the player must be a session admin
        if !player.is_session_admin {
            return Err(tonic::Status::permission_denied(
                "only session admin can pause game",
            ));
        }

        //notify all players of the change
        let res = session.notify_players_pause_game(&player).await;
        //convert into tonic::Status message
        res.map_err(|e| tonic::Status::internal(e.to_string()))?;
        Ok(tonic::Response::new(()))
    }
    async fn resume_game(
        &self,
        request: tonic::Request<proto::Player>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        let player = request.into_inner();
        let ses_id = ObjectId::from_str(&player.session_id)
            .with_context(|| format!("invalid session id: {}", player.session_id));
        //convert into tonic::Status message
        let ses_id = ses_id.map_err(|e| tonic::Status::invalid_argument(e.to_string()))?;
        let mut session = self.sessions.get_mut(&ses_id).unwrap();

        //the player must be a session admin
        if !player.is_session_admin {
            return Err(tonic::Status::permission_denied(
                "only session admin can resume game",
            ));
        }

        //notify all players of the change
        let res = session.notify_players_resume_game(&player).await;
        //convert into tonic::Status message
        res.map_err(|e| tonic::Status::internal(e.to_string()))?;
        Ok(tonic::Response::new(()))
    }
}
