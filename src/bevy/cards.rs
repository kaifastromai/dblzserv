use bevy::prelude::*;

///The card component
#[derive(Component)]
pub struct Card {
    pub color: crate::Color,
    ///the gender should be indicated by the eveness of the card number
    pub gender: crate::Gender,
    pub value: CardNumber,
}

impl From<crate::Card> for Card {
    fn from(value: crate::Card) -> Self {
        Self {
            color: value.color,
            gender: value.gender,
            value: (value.number as usize).into(),
        }
    }
}
///the card bundle
#[derive(Bundle)]
pub struct CardBundle {
    pub card: Card,
    sprite_bundle: SpriteBundle,
}
impl CardBundle {
    pub fn new(card: Card, texture: Handle<Image>, transform: Transform) -> Self {
        let sprite_bundle = SpriteBundle {
            transform,
            texture,
            ..Default::default()
        };
        Self {
            card,
            sprite_bundle,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CardNumber {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
}
impl From<CardNumber> for usize {
    fn from(card_number: CardNumber) -> Self {
        match card_number {
            CardNumber::One => 0,
            CardNumber::Two => 1,
            CardNumber::Three => 2,
            CardNumber::Four => 3,
            CardNumber::Five => 4,
            CardNumber::Six => 5,
            CardNumber::Seven => 6,
            CardNumber::Eight => 7,
            CardNumber::Nine => 8,
            CardNumber::Ten => 9,
        }
    }
}
impl From<usize> for CardNumber {
    fn from(index: usize) -> Self {
        match index {
            0 => CardNumber::One,
            1 => CardNumber::Two,
            2 => CardNumber::Three,
            3 => CardNumber::Four,
            4 => CardNumber::Five,
            5 => CardNumber::Six,
            6 => CardNumber::Seven,
            7 => CardNumber::Eight,
            8 => CardNumber::Nine,
            9 => CardNumber::Ten,
            _ => panic!("CardNumber::from(usize) called with index > 9"),
        }
    }
}
impl Card {
    pub fn as_index(&self) -> usize {
        let color_index = match self.color {
            crate::Color::Red => 0,
            crate::Color::Green => 1,
            crate::Color::Blue => 2,
            crate::Color::Yellow => 3,
        };
        // let gender_index = match self.gender {
        //     Gender::Boy => 0,
        //     Gender::Girl => 1,
        // };
        let value_index: usize = self.value.into();
        color_index * 10 + value_index
    }
}

pub struct CardMaterialRef {
    pub index: usize,
    pub handle: Handle<Image>,
}
#[derive(Resource)]
pub struct CardMaterialResource {
    ///a vector of all card materials used in the game.
    pub card_materials: Vec<CardMaterialRef>,
}
impl CardMaterialResource {
    pub fn get_material(&self, card: &Card) -> Handle<Image> {
        let index = card.as_index();
        self.card_materials[index].handle.clone()
    }
}
impl FromWorld for CardMaterialResource {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let mut card_refs = Vec::with_capacity(40);

        let vals = (0..40)
            .map(|i| {
                //we need the number format to be padded by 0s to 4 digits
                let image_path = format!("card_images/blitz_{i:04}.png");
                asset_server.load(image_path)
            })
            .collect::<Vec<_>>();

        let mut images = world.get_resource_mut::<Assets<Image>>().unwrap();
        for (i, handle) in vals.into_iter().enumerate() {
            card_refs.push(CardMaterialRef {
                index: i,
                handle: images.add(handle.into()),
            });
        }

        CardMaterialResource {
            card_materials: card_refs,
        }
    }
}
