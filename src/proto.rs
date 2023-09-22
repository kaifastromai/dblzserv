#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Card {
    #[prost(uint32, tag = "1")]
    pub player_id: u32,
    #[prost(uint32, tag = "2")]
    pub number: u32,
    #[prost(enumeration = "Color", tag = "3")]
    pub color: i32,
    #[prost(enumeration = "Gender", tag = "4")]
    pub gender: i32,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Player {
    /// A unique name for the player. Used to track stats outside of a particular session
    #[prost(string, tag = "1")]
    pub username: ::prost::alloc::string::String,
    /// the session the player is currently in
    #[prost(string, tag = "6")]
    pub session_id: ::prost::alloc::string::String,
    /// The id of player in the game. This is the index of the player in the players array
    #[prost(uint32, tag = "7")]
    pub player_game_id: u32,
    /// The image id of the face image that the player wants to use
    #[prost(uint32, tag = "4")]
    pub face_image_id: u32,
    /// A session admin can start and pause sessions, and also change the draw rate
    #[prost(bool, tag = "5")]
    pub is_session_admin: bool,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RegisterPlayerRq {
    #[prost(message, optional, tag = "1")]
    pub player: ::core::option::Option<Player>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Session {
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    #[prost(string, repeated, tag = "2")]
    pub players: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SessionRes {
    #[prost(message, repeated, tag = "1")]
    pub sessions: ::prost::alloc::vec::Vec<Session>,
}
/// This represents the global deck of the game. This is generated once by the server and sent to the clients once the game starts. The clients keep a local copy of this deck.
/// All proceeding references to the cards are then returns in indices to this deck
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GlobalDeck {
    #[prost(message, repeated, tag = "1")]
    pub cards: ::prost::alloc::vec::Vec<Card>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GamePrefs {
    #[prost(uint32, tag = "1")]
    pub draw_rate: u32,
    #[prost(uint32, tag = "2")]
    pub post_pile_size: u32,
    #[prost(uint32, tag = "3")]
    pub score_to_win: u32,
    #[prost(uint32, tag = "4")]
    pub blitz_deduction: u32,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StartGameEvent {
    #[prost(message, optional, tag = "1")]
    pub player: ::core::option::Option<Player>,
    #[prost(message, optional, tag = "2")]
    pub prefs: ::core::option::Option<GamePrefs>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FaceImage {
    #[prost(uint32, tag = "1")]
    pub id: u32,
    #[prost(bytes = "vec", tag = "2")]
    pub image: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag = "3")]
    pub content_type: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StartSessionRq {
    #[prost(string, tag = "1")]
    pub username: ::prost::alloc::string::String,
    #[prost(uint32, tag = "2")]
    pub face_image_id: u32,
}
/// sent by the client to the server to open the event stream.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClientInitOpenStream {
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ChangeDrawRateEvent {
    #[prost(uint32, tag = "1")]
    pub new_rate: u32,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClientEvent {
    /// The id of this event. When the server makes a response to this event, it will include this id. This allows the client to match the response to the request
    /// This must be unique for each event for each player
    #[prost(uint32, tag = "3")]
    pub event_id: u32,
    /// The id of the player who made this event
    #[prost(uint32, tag = "4")]
    pub player_id: u32,
    #[prost(oneof = "client_event::Event", tags = "1, 2, 5, 6, 7")]
    pub event: ::core::option::Option<client_event::Event>,
}
/// Nested message and enum types in `ClientEvent`.
pub mod client_event {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Event {
        #[prost(message, tag = "1")]
        Play(super::Play),
        #[prost(enumeration = "super::ClientGameStateAction", tag = "2")]
        StaticEvent(i32),
        /// message to send a message without any data. Used when first connecting to the server
        #[prost(message, tag = "5")]
        OpenStream(super::ClientInitOpenStream),
        /// Start the game. Closes this session from accepting new players. Must be called by the session admin
        #[prost(message, tag = "6")]
        StartGame(super::StartGameEvent),
        #[prost(message, tag = "7")]
        ChangeDrawRate(super::ChangeDrawRateEvent),
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ArenaStateChange {
    /// The action that was taken (add or remove)
    #[prost(enumeration = "StateChangeAction", tag = "1")]
    pub action: i32,
    /// The index of the card that was added or removed
    #[prost(uint32, tag = "2")]
    pub card: u32,
    /// The index of the pile that was changed
    #[prost(uint32, tag = "3")]
    pub pile_index: u32,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PlayerStateChange {
    #[prost(uint32, tag = "1")]
    pub player_id: u32,
    /// The type of change that was made
    #[prost(enumeration = "PlayerStateChangeType", tag = "2")]
    pub change_type: i32,
    /// The action that was taken (add or remove)
    #[prost(enumeration = "StateChangeAction", tag = "3")]
    pub action: i32,
    /// The index of the card that was added or removed
    #[prost(uint32, tag = "4")]
    pub card: u32,
}
/// Sent after the server receives a client event. This is used to acknowledge that the server received the event and finished processing it
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Acknowledge {
    /// The id of the event that this is acknowledging
    #[prost(uint32, tag = "1")]
    pub event_id: u32,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ServerStartGameEvent {
    #[prost(message, optional, tag = "1")]
    pub prefs: ::core::option::Option<GamePrefs>,
    #[prost(message, optional, tag = "2")]
    pub global_deck: ::core::option::Option<GlobalDeck>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GameStateChange {
    #[prost(message, repeated, tag = "1")]
    pub arena_state_changes: ::prost::alloc::vec::Vec<ArenaStateChange>,
    #[prost(message, repeated, tag = "2")]
    pub player_state_changes: ::prost::alloc::vec::Vec<PlayerStateChange>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ServerEvent {
    #[prost(oneof = "server_event::Event", tags = "1, 3, 5, 4, 6")]
    pub event: ::core::option::Option<server_event::Event>,
}
/// Nested message and enum types in `ServerEvent`.
pub mod server_event {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Event {
        #[prost(message, tag = "1")]
        GameStateChange(super::GameStateChange),
        #[prost(message, tag = "3")]
        Acknowledge(super::Acknowledge),
        #[prost(enumeration = "super::ServerGameStateAction", tag = "5")]
        ServerGameStateAction(i32),
        #[prost(message, tag = "4")]
        StartGame(super::ServerStartGameEvent),
        #[prost(message, tag = "6")]
        ChangeDrawRate(super::ChangeDrawRateEvent),
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ChangeDrawRateRq {
    #[prost(string, tag = "1")]
    pub player_id: ::prost::alloc::string::String,
    #[prost(uint32, tag = "2")]
    pub draw_rate: u32,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ResetDrawRateRq {
    #[prost(string, tag = "1")]
    pub player_id: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ArenaPlay {
    #[prost(enumeration = "ArenaPlayType", tag = "1")]
    pub play_type: i32,
    #[prost(uint32, optional, tag = "2")]
    pub from_index: ::core::option::Option<u32>,
    #[prost(uint32, optional, tag = "3")]
    pub to_index: ::core::option::Option<u32>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PlayerPlay {
    #[prost(enumeration = "PlayerPlayType", tag = "1")]
    pub play_type: i32,
    #[prost(uint32, optional, tag = "2")]
    pub post_index: ::core::option::Option<u32>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CallBlitz {
    #[prost(uint32, tag = "2")]
    pub player_index: u32,
}
/// an enum representing the types of plays that can be made
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Play {
    #[prost(uint32, tag = "4")]
    pub player_id: u32,
    #[prost(oneof = "play::Play", tags = "1, 2, 3")]
    pub play: ::core::option::Option<play::Play>,
}
/// Nested message and enum types in `Play`.
pub mod play {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Play {
        #[prost(message, tag = "1")]
        ArenaPlay(super::ArenaPlay),
        #[prost(message, tag = "2")]
        PlayerPlay(super::PlayerPlay),
        #[prost(message, tag = "3")]
        CallBlitz(super::CallBlitz),
    }
}
/// Try and join an active session. The face_image_id is the id of the face image that the player wants to use.
/// Must be chosen from the available face images before joining a sessions
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct JoinSessionRq {
    #[prost(string, tag = "1")]
    pub session_id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub username: ::prost::alloc::string::String,
    /// The image id of the face image that the player wants to use
    #[prost(uint32, tag = "4")]
    pub face_image_id: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Color {
    Red = 0,
    Blue = 1,
    Green = 2,
    Yellow = 3,
}
impl Color {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Color::Red => "RED",
            Color::Blue => "BLUE",
            Color::Green => "GREEN",
            Color::Yellow => "YELLOW",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "RED" => Some(Self::Red),
            "BLUE" => Some(Self::Blue),
            "GREEN" => Some(Self::Green),
            "YELLOW" => Some(Self::Yellow),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Gender {
    Boy = 0,
    Girl = 1,
}
impl Gender {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Gender::Boy => "BOY",
            Gender::Girl => "GIRL",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "BOY" => Some(Self::Boy),
            "GIRL" => Some(Self::Girl),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ClientGameStateAction {
    PauseGame = 0,
    ResumeGame = 1,
    ResetDrawRate = 3,
}
impl ClientGameStateAction {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            ClientGameStateAction::PauseGame => "PAUSE_GAME",
            ClientGameStateAction::ResumeGame => "RESUME_GAME",
            ClientGameStateAction::ResetDrawRate => "RESET_DRAW_RATE",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "PAUSE_GAME" => Some(Self::PauseGame),
            "RESUME_GAME" => Some(Self::ResumeGame),
            "RESET_DRAW_RATE" => Some(Self::ResetDrawRate),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum PlayerStateChangeType {
    BlitzPile = 0,
    AvailableHand = 1,
    PostPile = 2,
    ResetPlayerHand = 3,
    TransferHandToAvailable = 4,
    PlayerCallBlitz = 5,
}
impl PlayerStateChangeType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            PlayerStateChangeType::BlitzPile => "BLITZ_PILE",
            PlayerStateChangeType::AvailableHand => "AVAILABLE_HAND",
            PlayerStateChangeType::PostPile => "POST_PILE",
            PlayerStateChangeType::ResetPlayerHand => "RESET_PLAYER_HAND",
            PlayerStateChangeType::TransferHandToAvailable => {
                "TRANSFER_HAND_TO_AVAILABLE"
            }
            PlayerStateChangeType::PlayerCallBlitz => "PLAYER_CALL_BLITZ",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "BLITZ_PILE" => Some(Self::BlitzPile),
            "AVAILABLE_HAND" => Some(Self::AvailableHand),
            "POST_PILE" => Some(Self::PostPile),
            "RESET_PLAYER_HAND" => Some(Self::ResetPlayerHand),
            "TRANSFER_HAND_TO_AVAILABLE" => Some(Self::TransferHandToAvailable),
            "PLAYER_CALL_BLITZ" => Some(Self::PlayerCallBlitz),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum StateChangeAction {
    Add = 0,
    Remove = 1,
}
impl StateChangeAction {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            StateChangeAction::Add => "ADD",
            StateChangeAction::Remove => "REMOVE",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "ADD" => Some(Self::Add),
            "REMOVE" => Some(Self::Remove),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ServerGameStateAction {
    ServerPauseGame = 0,
    ServerResumeGame = 1,
    ServerGameOver = 3,
    ServerNewRound = 4,
}
impl ServerGameStateAction {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            ServerGameStateAction::ServerPauseGame => "SERVER_PAUSE_GAME",
            ServerGameStateAction::ServerResumeGame => "SERVER_RESUME_GAME",
            ServerGameStateAction::ServerGameOver => "SERVER_GAME_OVER",
            ServerGameStateAction::ServerNewRound => "SERVER_NEW_ROUND",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "SERVER_PAUSE_GAME" => Some(Self::ServerPauseGame),
            "SERVER_RESUME_GAME" => Some(Self::ServerResumeGame),
            "SERVER_GAME_OVER" => Some(Self::ServerGameOver),
            "SERVER_NEW_ROUND" => Some(Self::ServerNewRound),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ArenaPlayType {
    FromAvailableHand = 0,
    FromBlitz = 1,
    FromPost = 2,
}
impl ArenaPlayType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            ArenaPlayType::FromAvailableHand => "FROM_AVAILABLE_HAND",
            ArenaPlayType::FromBlitz => "FROM_BLITZ",
            ArenaPlayType::FromPost => "FROM_POST",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "FROM_AVAILABLE_HAND" => Some(Self::FromAvailableHand),
            "FROM_BLITZ" => Some(Self::FromBlitz),
            "FROM_POST" => Some(Self::FromPost),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum PlayerPlayType {
    BlitzToPost = 0,
    AvailableHandToPost = 1,
    TransferToAvailableHand = 2,
    ResetHand = 3,
}
impl PlayerPlayType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            PlayerPlayType::BlitzToPost => "BLITZ_TO_POST",
            PlayerPlayType::AvailableHandToPost => "AVAILABLE_HAND_TO_POST",
            PlayerPlayType::TransferToAvailableHand => "TRANSFER_TO_AVAILABLE_HAND",
            PlayerPlayType::ResetHand => "RESET_HAND",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "BLITZ_TO_POST" => Some(Self::BlitzToPost),
            "AVAILABLE_HAND_TO_POST" => Some(Self::AvailableHandToPost),
            "TRANSFER_TO_AVAILABLE_HAND" => Some(Self::TransferToAvailableHand),
            "RESET_HAND" => Some(Self::ResetHand),
            _ => None,
        }
    }
}
/// Generated client implementations.
pub mod session_service_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    #[derive(Debug, Clone)]
    pub struct SessionServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl SessionServiceClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> SessionServiceClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> SessionServiceClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            SessionServiceClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        pub async fn get_active_sessions(
            &mut self,
            request: impl tonic::IntoRequest<()>,
        ) -> std::result::Result<tonic::Response<super::SessionRes>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proto.SessionService/GetActiveSessions",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("proto.SessionService", "GetActiveSessions"));
            self.inner.unary(req, path, codec).await
        }
        /// Join an active session
        pub async fn join_session(
            &mut self,
            request: impl tonic::IntoRequest<super::JoinSessionRq>,
        ) -> std::result::Result<tonic::Response<super::Player>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proto.SessionService/JoinSession",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("proto.SessionService", "JoinSession"));
            self.inner.unary(req, path, codec).await
        }
        /// Start a new session that other players can join. The player who starts the session is the session admin. It then returns a Player message
        /// with the player_id field set to the player's unique identifier
        pub async fn start_session(
            &mut self,
            request: impl tonic::IntoRequest<super::StartSessionRq>,
        ) -> std::result::Result<tonic::Response<super::Player>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proto.SessionService/StartSession",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("proto.SessionService", "StartSession"));
            self.inner.unary(req, path, codec).await
        }
        /// End the given session. Must be called by admin
        pub async fn end_session(
            &mut self,
            request: impl tonic::IntoRequest<super::Player>,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proto.SessionService/EndSession",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("proto.SessionService", "EndSession"));
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated client implementations.
pub mod image_service_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// Manages the possible face images that the players can choose from
    #[derive(Debug, Clone)]
    pub struct ImageServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl ImageServiceClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> ImageServiceClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> ImageServiceClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            ImageServiceClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        pub async fn get_face_images(
            &mut self,
            request: impl tonic::IntoRequest<()>,
        ) -> std::result::Result<tonic::Response<super::FaceImage>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proto.ImageService/GetFaceImages",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("proto.ImageService", "GetFaceImages"));
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated client implementations.
pub mod game_service_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// Handles communication within a game session
    #[derive(Debug, Clone)]
    pub struct GameServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl GameServiceClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> GameServiceClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> GameServiceClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            GameServiceClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        /// Open the main event stream. This allows the client and server to communicate with each other
        pub async fn open_event_stream(
            &mut self,
            request: impl tonic::IntoStreamingRequest<Message = super::ClientEvent>,
        ) -> std::result::Result<
            tonic::Response<tonic::codec::Streaming<super::ServerEvent>>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/proto.GameService/OpenEventStream",
            );
            let mut req = request.into_streaming_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("proto.GameService", "OpenEventStream"));
            self.inner.streaming(req, path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod session_service_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with SessionServiceServer.
    #[async_trait]
    pub trait SessionService: Send + Sync + 'static {
        async fn get_active_sessions(
            &self,
            request: tonic::Request<()>,
        ) -> std::result::Result<tonic::Response<super::SessionRes>, tonic::Status>;
        /// Join an active session
        async fn join_session(
            &self,
            request: tonic::Request<super::JoinSessionRq>,
        ) -> std::result::Result<tonic::Response<super::Player>, tonic::Status>;
        /// Start a new session that other players can join. The player who starts the session is the session admin. It then returns a Player message
        /// with the player_id field set to the player's unique identifier
        async fn start_session(
            &self,
            request: tonic::Request<super::StartSessionRq>,
        ) -> std::result::Result<tonic::Response<super::Player>, tonic::Status>;
        /// End the given session. Must be called by admin
        async fn end_session(
            &self,
            request: tonic::Request<super::Player>,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct SessionServiceServer<T: SessionService> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: SessionService> SessionServiceServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for SessionServiceServer<T>
    where
        T: SessionService,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/proto.SessionService/GetActiveSessions" => {
                    #[allow(non_camel_case_types)]
                    struct GetActiveSessionsSvc<T: SessionService>(pub Arc<T>);
                    impl<T: SessionService> tonic::server::UnaryService<()>
                    for GetActiveSessionsSvc<T> {
                        type Response = super::SessionRes;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(&mut self, request: tonic::Request<()>) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SessionService>::get_active_sessions(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetActiveSessionsSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.SessionService/JoinSession" => {
                    #[allow(non_camel_case_types)]
                    struct JoinSessionSvc<T: SessionService>(pub Arc<T>);
                    impl<
                        T: SessionService,
                    > tonic::server::UnaryService<super::JoinSessionRq>
                    for JoinSessionSvc<T> {
                        type Response = super::Player;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::JoinSessionRq>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SessionService>::join_session(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = JoinSessionSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.SessionService/StartSession" => {
                    #[allow(non_camel_case_types)]
                    struct StartSessionSvc<T: SessionService>(pub Arc<T>);
                    impl<
                        T: SessionService,
                    > tonic::server::UnaryService<super::StartSessionRq>
                    for StartSessionSvc<T> {
                        type Response = super::Player;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::StartSessionRq>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SessionService>::start_session(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = StartSessionSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/proto.SessionService/EndSession" => {
                    #[allow(non_camel_case_types)]
                    struct EndSessionSvc<T: SessionService>(pub Arc<T>);
                    impl<T: SessionService> tonic::server::UnaryService<super::Player>
                    for EndSessionSvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::Player>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SessionService>::end_session(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = EndSessionSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: SessionService> Clone for SessionServiceServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    impl<T: SessionService> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: SessionService> tonic::server::NamedService for SessionServiceServer<T> {
        const NAME: &'static str = "proto.SessionService";
    }
}
/// Generated server implementations.
pub mod image_service_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with ImageServiceServer.
    #[async_trait]
    pub trait ImageService: Send + Sync + 'static {
        async fn get_face_images(
            &self,
            request: tonic::Request<()>,
        ) -> std::result::Result<tonic::Response<super::FaceImage>, tonic::Status>;
    }
    /// Manages the possible face images that the players can choose from
    #[derive(Debug)]
    pub struct ImageServiceServer<T: ImageService> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: ImageService> ImageServiceServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for ImageServiceServer<T>
    where
        T: ImageService,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/proto.ImageService/GetFaceImages" => {
                    #[allow(non_camel_case_types)]
                    struct GetFaceImagesSvc<T: ImageService>(pub Arc<T>);
                    impl<T: ImageService> tonic::server::UnaryService<()>
                    for GetFaceImagesSvc<T> {
                        type Response = super::FaceImage;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(&mut self, request: tonic::Request<()>) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ImageService>::get_face_images(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetFaceImagesSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: ImageService> Clone for ImageServiceServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    impl<T: ImageService> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: ImageService> tonic::server::NamedService for ImageServiceServer<T> {
        const NAME: &'static str = "proto.ImageService";
    }
}
/// Generated server implementations.
pub mod game_service_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with GameServiceServer.
    #[async_trait]
    pub trait GameService: Send + Sync + 'static {
        /// Server streaming response type for the OpenEventStream method.
        type OpenEventStreamStream: tonic::codegen::tokio_stream::Stream<
                Item = std::result::Result<super::ServerEvent, tonic::Status>,
            >
            + Send
            + 'static;
        /// Open the main event stream. This allows the client and server to communicate with each other
        async fn open_event_stream(
            &self,
            request: tonic::Request<tonic::Streaming<super::ClientEvent>>,
        ) -> std::result::Result<
            tonic::Response<Self::OpenEventStreamStream>,
            tonic::Status,
        >;
    }
    /// Handles communication within a game session
    #[derive(Debug)]
    pub struct GameServiceServer<T: GameService> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: GameService> GameServiceServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for GameServiceServer<T>
    where
        T: GameService,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/proto.GameService/OpenEventStream" => {
                    #[allow(non_camel_case_types)]
                    struct OpenEventStreamSvc<T: GameService>(pub Arc<T>);
                    impl<
                        T: GameService,
                    > tonic::server::StreamingService<super::ClientEvent>
                    for OpenEventStreamSvc<T> {
                        type Response = super::ServerEvent;
                        type ResponseStream = T::OpenEventStreamStream;
                        type Future = BoxFuture<
                            tonic::Response<Self::ResponseStream>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<tonic::Streaming<super::ClientEvent>>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as GameService>::open_event_stream(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = OpenEventStreamSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.streaming(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: GameService> Clone for GameServiceServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    impl<T: GameService> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: GameService> tonic::server::NamedService for GameServiceServer<T> {
        const NAME: &'static str = "proto.GameService";
    }
}
