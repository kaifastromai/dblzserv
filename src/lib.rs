pub mod server;

pub mod proto;
use anyhow::{anyhow, Context, Result};
use proto::{ArenaStateChange, GameStateChange, PlayerStateChange, ServerGameStateAction};
use rand::seq::SliceRandom;

///Represents a card in the game. It is very similar to normal playing cards, with some differences.
/// Each card can have a number 1-10, a color, and a gender (boy or girl), and an id (which is associated with the 'face'/image in the original game (and in the client)).
/// When the server is running, we maintain an array of all possible cards, and each card is identified by its index in the array.
#[derive(Clone, Copy, Debug)]
pub struct Card {
    pub player_id: u32,
    pub number: u32,
    pub color: Color,
    pub gender: Gender,
}

///Shuffle cards in place
pub fn shuffle(cards: &mut Vec<u32>) -> &mut Vec<u32> {
    let mut rng = rand::thread_rng();
    let cards = cards;
    cards.shuffle(&mut rng);
    cards
}

#[derive(Clone, Copy, Debug)]
pub enum Action {
    Arena(ArenaAction),
    Player(PlayerAction),
    ///Calls blitz. If called by a player and another player can call blitz (their blitz pile is empty) but has not, then
    /// the player on which blitz was called loses 10 points.
    CallBlitz(u32),
}
#[derive(Clone, Copy, Debug)]
///Plays that transfer cards from a player's hand to the arena.
pub enum ArenaAction {
    FromAvailableHand(u32),
    FromBlitz(u32),

    ///the post pile to take from and the arena pile to put on
    FromPost {
        post_pile: u32,
        arena_pile: u32,
    },
}

///Plays that modify the players own cards
#[derive(Clone, Copy, Debug)]
pub enum PlayerAction {
    BlitzToPost(u32),
    AvailableToPost(u32),
    TransferToAvailable,
    ResetHand,
}
///Represents the types of plays that can be made by a player.
#[derive(Clone, Copy, Debug)]
pub struct Play {
    pub player: u32,
    pub play: Action,
}
///represents a player in the game.
pub struct Player {
    pub player_id: u32,
    pub hand: PlayerHand,
    pub post_pile: PostPile,
    pub blitz_pile: BlitzPile,
}
impl Player {
    pub fn can_call_blitz(&self) -> bool {
        self.blitz_pile.can_call_blitz()
    }
}

pub struct GameStateBuilder {
    pub draw_rate: u32,
    pub post_pile_size: u32,
    pub player_count: u32,
    pub score_to_win: u32,
    pub blitz_deduction: u32,
}
impl GameStateBuilder {
    pub fn new() -> Self {
        Self {
            draw_rate: 3,
            post_pile_size: 3,
            player_count: 2,
            score_to_win: 72,
            blitz_deduction: 10,
        }
    }
    pub fn with_draw_rate(mut self, draw_rate: u32) -> Self {
        self.draw_rate = draw_rate;
        self
    }
    pub fn with_post_pile_size(mut self, post_pile_size: u32) -> Self {
        self.post_pile_size = post_pile_size;
        self
    }
    pub fn with_player_count(mut self, player_count: u32) -> Self {
        self.player_count = player_count;
        self
    }
    pub fn with_score_to_win(mut self, score_to_win: u32) -> Self {
        self.score_to_win = score_to_win;
        self
    }
    pub fn with_blitz_deduction(mut self, blitz_deduction: u32) -> Self {
        self.blitz_deduction = blitz_deduction;
        self
    }
    pub fn build(self) -> Result<GameState> {
        GameState::from_build(self)
    }
}

impl Default for GameStateBuilder {
    fn default() -> Self {
        Self::new()
    }
}
pub struct GameState {
    pub round: u32,
    pub scoreboard: Scoreboard,
    pub card_context: CardContext,
    pub players: Vec<Player>,
    pub arena: Arena,
    pub draw_rate: u32,
    pub post_pile_size: u32,
    ///The score a player needs to win the game. Defaults to 72
    pub score_to_win: u32,
    ///Amount of points to deduct if someone calls blitz on a player who can call blitz but has not.xs
    pub blitz_deduction: u32,
    default_draw_rate: u32,
    is_game_over: bool,
}
impl GameState {
    pub fn new(
        player_count: u32,
        proto::GamePrefs {
            draw_rate,
            post_pile_size,
            score_to_win,
            blitz_deduction,
        }: proto::GamePrefs,
    ) -> Result<GameState> {
        let cards = generate_all_card(player_count);
        let card_context = CardContext::new(cards);
        //Once we have all the cards, we need to get player hands.
        let mut players = Vec::new();
        players.reserve(player_count as usize);

        for i in 0..player_count {
            //each player gets a 40 card hand. From the hand post_pile_size cards are removed and placed in the post pile,
            // 10 cards are removed and placed in the blitz pile.
            //the rest of the cards are placed in the player's hand.
            let mut player_cards: Vec<u32> = card_context
                .cards
                .iter()
                .enumerate()
                .skip((i * 40) as usize)
                .take(40)
                .map(|(i, _c)| i as u32)
                .collect();
            shuffle(&mut player_cards);

            let post_piles = player_cards
                .iter()
                .skip((40 - post_pile_size) as usize)
                .take(post_pile_size as usize)
                .copied()
                .map(|i| {
                    let card = card_context.cards[i as usize];
                    Pile::from_vec(vec![i], card.color)
                })
                .collect::<Vec<_>>();
            let blitz_pile = player_cards
                .iter()
                .skip((40 - post_pile_size - 10) as usize)
                .take(10)
                .copied()
                .collect::<Vec<_>>();
            let hand = player_cards
                .iter()
                .take((40 - post_pile_size - 10) as usize)
                .copied()
                .collect::<Vec<_>>();
            players.push(Player {
                player_id: i,
                hand: PlayerHand::new(hand),
                post_pile: PostPile { piles: post_piles },
                blitz_pile: BlitzPile::new(blitz_pile),
            });
        }

        //the arena is initially empty.
        let arena = Arena::new();
        Ok(GameState {
            card_context,
            players,
            arena,
            draw_rate,
            round: 0,
            scoreboard: Scoreboard::new(player_count),
            post_pile_size,
            score_to_win,
            blitz_deduction,
            default_draw_rate: draw_rate,
            is_game_over: false,
        })
    }

    pub fn from_build(builder: GameStateBuilder) -> Result<GameState> {
        GameState::new(
            builder.player_count,
            proto::GamePrefs {
                post_pile_size: builder.post_pile_size,
                score_to_win: builder.score_to_win,
                blitz_deduction: builder.blitz_deduction,
                draw_rate: builder.draw_rate,
            },
        )
    }
    pub fn create_player(&self, player_id: u32) -> Result<Player> {
        //each player gets a 40 card hand. From the hand post_pile_size cards are removed and placed in the post pile,
        // 10 cards are removed and placed in the blitz pile.
        //the rest of the cards are placed in the player's hand.
        let mut player_cards: Vec<u32> = self
            .card_context
            .cards
            .iter()
            .enumerate()
            .skip((player_id * 40) as usize)
            .take(40)
            .map(|(i, _c)| i as u32)
            .collect();
        shuffle(&mut player_cards);

        let post_piles = player_cards
            .iter()
            .skip((40 - self.post_pile_size) as usize)
            .take(self.post_pile_size as usize)
            .copied()
            .map(|i| {
                let card = self.card_context.cards[i as usize];
                Pile::from_vec(vec![i], card.color)
            })
            .collect::<Vec<_>>();
        let blitz_pile = player_cards
            .iter()
            .skip((40 - self.post_pile_size - 10) as usize)
            .take(10)
            .copied()
            .collect::<Vec<_>>();
        let hand = player_cards
            .iter()
            .take((40 - self.post_pile_size - 10) as usize)
            .copied()
            .collect::<Vec<_>>();
        Ok(Player {
            player_id,
            hand: PlayerHand::new(hand),
            post_pile: PostPile { piles: post_piles },
            blitz_pile: BlitzPile::new(blitz_pile),
        })
    }

    pub fn new_round(&mut self) -> Result<()> {
        self.round += 1;
        //clear arena
        self.arena.piles.clear();
        //clear players
        for player in self.players.iter_mut() {
            player.blitz_pile.clear();
            player.post_pile.clear();
            player.hand.clear();
        }
        //create new players
        self.players = (0..self.players.len())
            .map(|i| self.create_player(i as u32).unwrap())
            .collect();
        Ok(())
    }

    //Make a play. Emits an event describing whether a card was added/deleted to/from the arena, or whether a player's hand was modified.
    pub fn make_play(&mut self, play: Play) -> Result<proto::server_event::Event> {
        let player = play.player;

        let event = match play.play {
            Action::Arena(p) => match p {
                ArenaAction::FromAvailableHand(pile) => {
                    let card = self.players[player as usize].hand.play_from_available()?;
                    self.arena.add_card(pile, card, &mut self.card_context)?;
                    //emit event
                    let event = proto::server_event::Event::GameStateChange(GameStateChange {
                        arena_state_changes: vec![ArenaStateChange {
                            action: proto::StateChangeAction::Add as i32,
                            card,
                            pile_index: pile,
                        }],
                        player_state_changes: vec![PlayerStateChange {
                            player_id: player,
                            change_type: proto::PlayerStateChangeType::AvailableHand as i32,
                            action: proto::StateChangeAction::Remove as i32,
                            card,
                        }],
                    });
                    event
                }
                ArenaAction::FromBlitz(pile) => {
                    let card = self.players[player as usize].blitz_pile.play()?;
                    self.arena.add_card(pile, card, &mut self.card_context)?;

                    //emit event
                    let event = proto::server_event::Event::GameStateChange(GameStateChange {
                        arena_state_changes: vec![ArenaStateChange {
                            action: proto::StateChangeAction::Add as i32,
                            card,
                            pile_index: pile,
                        }],
                        player_state_changes: vec![PlayerStateChange {
                            player_id: player,
                            change_type: proto::PlayerStateChangeType::BlitzPile as i32,
                            action: proto::StateChangeAction::Remove as i32,
                            card,
                        }],
                    });
                    event
                }

                ArenaAction::FromPost {
                    post_pile,
                    arena_pile,
                } => {
                    let card = self.players[player as usize].post_pile.play(post_pile)?;
                    self.arena
                        .add_card(arena_pile, card, &mut self.card_context)?;
                    //emit event
                    let event = proto::server_event::Event::GameStateChange(GameStateChange {
                        arena_state_changes: vec![ArenaStateChange {
                            action: proto::StateChangeAction::Add as i32,
                            card,
                            pile_index: post_pile,
                        }],
                        player_state_changes: vec![PlayerStateChange {
                            player_id: player,
                            change_type: proto::PlayerStateChangeType::PostPile as i32,
                            action: proto::StateChangeAction::Remove as i32,
                            card,
                        }],
                    });
                    event
                }
            },
            Action::Player(p) => {
                match p {
                    PlayerAction::BlitzToPost(p) => {
                        let blitz_card = self.players[player as usize].blitz_pile.play()?;
                        //add to post pile at position p
                        self.players[player as usize].post_pile.add_card(
                            p,
                            blitz_card,
                            &mut self.card_context,
                        )?;
                        //emit event
                        let event = proto::server_event::Event::GameStateChange(GameStateChange {
                            arena_state_changes: vec![],
                            player_state_changes: vec![
                                PlayerStateChange {
                                    player_id: player,
                                    change_type: proto::PlayerStateChangeType::BlitzPile as i32,
                                    action: proto::StateChangeAction::Remove as i32,
                                    card: blitz_card,
                                },
                                PlayerStateChange {
                                    player_id: player,
                                    change_type: proto::PlayerStateChangeType::PostPile as i32,
                                    action: proto::StateChangeAction::Add as i32,
                                    card: blitz_card,
                                },
                            ],
                        });
                        event
                    }
                    PlayerAction::AvailableToPost(u32) => {
                        let card = self.players[player as usize].hand.play_from_available()?;
                        self.players[player as usize].post_pile.add_card(
                            u32,
                            card,
                            &mut self.card_context,
                        )?;

                        //emit event
                        let event = proto::server_event::Event::GameStateChange(GameStateChange {
                            arena_state_changes: vec![],
                            player_state_changes: vec![
                                PlayerStateChange {
                                    player_id: player,
                                    change_type: proto::PlayerStateChangeType::AvailableHand as i32,
                                    action: proto::StateChangeAction::Remove as i32,
                                    card: card,
                                },
                                PlayerStateChange {
                                    player_id: player,
                                    change_type: proto::PlayerStateChangeType::PostPile as i32,
                                    action: proto::StateChangeAction::Add as i32,
                                    card: card,
                                },
                            ],
                        });
                        event
                    }
                    PlayerAction::TransferToAvailable => {
                        self.players[player as usize]
                            .hand
                            .transfer_hand_to_available(self.draw_rate);
                        let event = proto::server_event::Event::GameStateChange(GameStateChange {
                            arena_state_changes: vec![],
                            player_state_changes: vec![PlayerStateChange {
                                player_id: player,
                                change_type: proto::PlayerStateChangeType::TransferHandToAvailable
                                    as i32,
                                action: proto::StateChangeAction::Remove as i32,
                                card: 0,
                            }],
                        });
                        event
                    }
                    PlayerAction::ResetHand => {
                        self.players[player as usize].hand.reset_hand();
                        let event = proto::server_event::Event::GameStateChange(GameStateChange {
                            arena_state_changes: vec![],
                            player_state_changes: vec![PlayerStateChange {
                                player_id: player,
                                change_type: proto::PlayerStateChangeType::ResetPlayerHand as i32,
                                action: proto::StateChangeAction::Remove as i32,
                                card: 0,
                            }],
                        });
                        event
                    }
                }
            }
            Action::CallBlitz(_p) => {
                //when blitz is called,we count up all the cards in the arena, and give players points depending upon how many cards they played.
                if self.players[player as usize].can_call_blitz() {
                    //everything is normal, new round
                    self.score_round();
                    self.new_round()?;
                    proto::server_event::Event::ServerGameStateAction(
                        ServerGameStateAction::ServerNewRound as i32,
                    )
                } else {
                    let blitzed_players: Vec<u32> = self
                        .players
                        .iter()
                        .enumerate()
                        .filter(|(i, p)| p.can_call_blitz() && *i != player as usize)
                        .map(|(i, _)| i as u32)
                        .collect();

                    for p in blitzed_players {
                        //these players get blitz_deduction points deducted from their score.
                        //deduct points from the player
                        self.scoreboard
                            .add_score(self.round, p, -(self.blitz_deduction as i32));
                    }
                    self.score_round();
                    proto::server_event::Event::ServerGameStateAction(
                        ServerGameStateAction::ServerGameOver as i32,
                    )
                }
            }
        };
        Ok(event)
    }

    ///Counts up all the cards in the arena, and gives players points depending upon how many cards they played. Called at the end of a round (when blitz is called).
    /// We also count up how many cards are left in the blitz pile and subtract 2* that number from the player's score.
    pub fn score_round(&mut self) {
        let mut player_arena_scores = vec![0; self.players.len()];
        for pile in self.arena.piles.iter() {
            for card in pile.cards.iter() {
                let card = self.card_context.cards[*card as usize];
                player_arena_scores[card.player_id as usize] += 1;
            }
        }
        let blitz_scores = self
            .players
            .iter()
            .map(|p| p.blitz_pile.cards.len() as i32 * -2)
            .collect::<Vec<_>>();
        player_arena_scores = player_arena_scores
            .iter()
            .zip(blitz_scores.iter())
            .map(|(a, b)| a + b)
            .collect();
        self.scoreboard.add_round(self.round, player_arena_scores);
        //if any player has a score equal to or greater than the win score, the game is over.
        let player_total_scores = self.scoreboard.get_totals();
        if player_total_scores
            .iter()
            .any(|s| *s >= self.score_to_win as i32)
        {
            self.is_game_over = true;
        }
    }
    pub fn change_draw_rate(&mut self, new_rate: u32) {
        self.draw_rate = new_rate;
    }
    pub fn reset_draw_rate(&mut self) {
        self.draw_rate = self.default_draw_rate;
    }
}

pub struct Scoreboard {
    //holds per round scores for each player.
    pub scores: Vec<Vec<i32>>,
}
impl Scoreboard {
    pub fn new(player_count: u32) -> Scoreboard {
        let mut scores = Vec::new();
        scores.reserve(player_count as usize);
        for _ in 0..player_count {
            scores.push(vec![]);
        }
        Scoreboard { scores }
    }
    pub fn get_totals(&self) -> Vec<i32> {
        let mut totals = Vec::new();
        totals.reserve(self.scores.len());
        for score in &self.scores {
            totals.push(score.iter().sum());
        }
        totals
    }
    pub fn add_round(&mut self, _round: u32, scores: Vec<i32>) {
        for (i, score) in scores.into_iter().enumerate() {
            self.scores[i].push(score);
        }
    }
    pub fn add_score(&mut self, round: u32, player: u32, score: i32) {
        self.scores[round as usize][player as usize] += score;
    }
}

///a pile is a stack of less <=10 cards.
pub struct Pile {
    pub cards: Vec<u32>,
    pub color: Color,
}
impl Pile {
    pub fn from_vec(cards: Vec<u32>, color: Color) -> Pile {
        Pile { cards, color }
    }

    pub fn add_arena_card(&mut self, card_index: u32, context: &mut CardContext) -> Result<()> {
        if self.cards.len() == 10 {
            return Err(anyhow!("Pile is full"));
        }

        let card = context.get_card(card_index as usize)?;
        if card.color != self.color {
            return Err(anyhow!("Card color does not match pile color"));
        }

        if card.number != (self.cards.len() + 1) as u32 {
            return Err(anyhow!(
                "Card number {} does not match pile counter {}",
                card.number,
                self.cards.len()
            ));
        }
        self.cards.push(card_index);
        Ok(())
    }
    ///When stacking on the post pile, the card must be the same color, the natural anteceeding number,and the gender must be the opposite of the previous card.
    pub fn add_post_card(&mut self, card_index: u32, context: &mut CardContext) -> Result<()> {
        if self.cards.len() == 10 {
            return Err(anyhow!("Pile is full"));
        }

        let card = context.get_card(card_index as usize)?;
        if card.color != self.color {
            return Err(anyhow!("Card color does not match pile color"));
        }
        let prev_card = context.get_card(self.cards[(self.cards.len() - 1)] as usize)?;
        //genders must not be the same
        if card.gender == prev_card.gender {
            return Err(anyhow!("Genders must alternate"));
        }
        if card.number != (prev_card.number - 1) {
            return Err(anyhow!("Card number does not match pile counter"));
        }

        self.cards[card.number as usize] = card_index;

        Ok(())
    }
}

///The context holds all the created cards
pub struct CardContext {
    cards: Vec<Card>,
}
impl CardContext {
    pub fn new(cards: Vec<Card>) -> CardContext {
        CardContext { cards }
    }
    pub fn get_card(&self, index: usize) -> Result<&Card> {
        self.cards
            .get(index)
            .ok_or_else(|| anyhow!("Card index out of bounds"))
    }
}

///The arena is the place where the players layout their cards (called "Dutch pile" in the original game). The aim is for the player
/// to stack the cards in same-color sequential order.
pub struct Arena {
    pub piles: Vec<Pile>,
}

impl Arena {
    pub fn new() -> Arena {
        Arena { piles: vec![] }
    }
    pub fn add_card(
        &mut self,
        pile_index: u32,
        card_index: u32,
        context: &mut CardContext,
    ) -> Result<()> {
        //add a card to a pile, or create a new one if the card number==1, in the case of a new pile the number must be 1
        let card = context.get_card(card_index as usize)?;
        if card.number == 1 {
            self.piles
                .push(Pile::from_vec(vec![card_index], card.color));
        } else {
            self.piles
                .get_mut(pile_index as usize)
                .with_context(|| {
                    format!(
                        "Pile index {} out of bounds when adding card {:?}",
                        pile_index, card
                    )
                })?
                .add_arena_card(card_index, context)?;
        }
        Ok(())
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

///The player hand contains two list of cards, one that the player is currently holding, and the other a stack of available cards.
/// They draw some amount of cards from their hand (3 usually), and then adds them to the available cards. The player can only play into the arena from the available cards.
pub struct PlayerHand {
    pub in_hand: Vec<u32>,
    pub available_to_play: Vec<u32>,
}
impl PlayerHand {
    pub fn new(cards: Vec<u32>) -> PlayerHand {
        PlayerHand {
            in_hand: cards,
            available_to_play: vec![],
        }
    }
    ///Transfers from in hand to the available_to_play pile.
    pub fn transfer_hand_to_available(&mut self, count: u32) -> Vec<u32> {
        let drawn = self.in_hand.drain(..count as usize);
        let vals: Vec<u32> = drawn.collect();
        self.available_to_play.extend(vals.iter());
        vals
    }
    ///Plays a card from the available pile, and returns the index of the card.
    /// If there are no cards in the available pile, returns an error.
    pub fn play_from_available(&mut self) -> Result<u32> {
        self.available_to_play
            .pop()
            .ok_or_else(|| anyhow!("No available cards to play"))
    }
    pub fn reset_hand(&mut self) {
        self.in_hand.append(&mut self.available_to_play);
        self.in_hand.clear();
    }
    pub fn clear(&mut self) {
        self.in_hand.clear();
        self.available_to_play.clear();
    }
    pub fn count_in_hand(&self) -> usize {
        self.in_hand.len()
    }
    pub fn count_available(&self) -> usize {
        self.available_to_play.len()
    }
}

///the post pile is a set of 3 or 5 piles.
/// In a game of 3 or less players, there are usually 5 post piles, but in a game of 4 or more players, there are 3.
/// Players can stack cards on the post pile, but it must go in descending order and the genders must swap
pub struct PostPile {
    pub piles: Vec<Pile>,
}
impl PostPile {
    pub fn new() -> PostPile {
        PostPile { piles: vec![] }
    }
    pub fn from_vec(cards: Vec<Pile>) -> PostPile {
        PostPile { piles: cards }
    }
    pub fn add_card(
        &mut self,
        pile_index: u32,
        card_index: u32,
        context: &mut CardContext,
    ) -> Result<()> {
        self.piles[pile_index as usize].add_post_card(card_index, context)
    }
    ///Plays the top card from the post pile. If the card is the last card in the pile, the pile is removed.
    pub fn play(&mut self, pile_index: u32) -> Result<u32> {
        let pile = self
            .piles
            .get_mut(pile_index as usize)
            .ok_or_else(|| anyhow!("Pile index out of bounds"))?;
        let card = pile.cards.pop().ok_or_else(|| anyhow!("Pile is empty"))?;

        if pile.cards.is_empty() {
            self.piles.remove(pile_index as usize);
        }
        Ok(card)
    }
    pub fn clear(&mut self) {
        self.piles.clear();
    }
}

impl Default for PostPile {
    fn default() -> Self {
        Self::new()
    }
}

///The BlitzPile is a pile of 10 cards dealt from the players main hand at the start of the game. If the player gets rid of all the cards in the blitz pile, the round ends.
pub struct BlitzPile {
    pub cards: Vec<u32>,
}
impl BlitzPile {
    pub fn new(cards: Vec<u32>) -> BlitzPile {
        BlitzPile { cards }
    }
    ///Play the top card from the blitz pile.
    pub fn play(&mut self) -> Result<u32> {
        let card = self
            .cards
            .pop()
            .ok_or_else(|| anyhow!("Blitz pile is empty"))?;
        Ok(card)
    }
    pub fn can_call_blitz(&self) -> bool {
        self.cards.is_empty()
    }
    pub fn clear(&mut self) {
        self.cards.clear();
    }
}
///There are only four colors in the game: red, blue, green, and yellow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Color {
    Red = 0,
    Blue = 1,
    Green = 2,
    Yellow = 3,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum Gender {
    Boy = 0,
    Girl = 1,
}

///Genearte all possible cards for this game given the player count.
pub fn generate_all_card(players: u32) -> Vec<Card> {
    let colors = [Color::Red, Color::Blue, Color::Green, Color::Yellow];
    let mut cards = vec![
        Card {
            player_id: 0,
            number: 0,
            color: Color::Red,
            gender: Gender::Boy
        };
        40 * players as usize
    ];
    for player in 0..players {
        for n in 0..40 {
            cards[(n + 40 * player) as usize] = Card {
                player_id: player,
                number: (n % 10 + 1),
                color: colors[((n) / 10) as usize],
                gender: match n % 2 {
                    0 => Gender::Boy,
                    1 => Gender::Girl,
                    _ => unreachable!(),
                },
            };
        }
    }

    cards
}
