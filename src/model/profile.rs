use crate::{LodestoneError, ServerParseError};
use select::document::Document;
use select::predicate::{Class, Name};
use std::num::ParseIntError;
use std::str::FromStr;
use thiserror::Error;

use crate::model::clan::ClanParseError;
use crate::model::class::ClassTypeParseError;
use crate::model::gender::GenderParseError;
use crate::model::race::RaceParseError;
use crate::model::{
    attribute::{Attribute, Attributes},
    clan::Clan,
    class::{ClassInfo, ClassType, Classes},
    gender::Gender,
    race::Race,
    server::Server,
};

use crate::model::util::load_profile_url_async;
#[cfg(blocking)]
use crate::model::util::load_url;

/// Represents ways in which a search over the HTML data might go wrong.
#[derive(Error, Debug)]
pub enum SearchError {
    /// A search for a node that was required turned up empty.
    #[error("Node not found: {0}")]
    NodeNotFound(&'static str),
    /// A node was found, but the data inside it was malformed.
    #[error("Invalid data found while parsing '{0}'")]
    InvalidData(&'static str),
    #[error("Invalid server {0}")]
    ServerParseError(#[from] ServerParseError),
    #[error("Clan parse error {0}")]
    ClanParseError(#[from] ClanParseError),
    #[error("Gender parse error {0}")]
    GenderParseError(#[from] GenderParseError),
    #[error("{0}")]
    ParseIntError(#[from] ParseIntError),
    #[error("class type error {0}")]
    ClassTypeError(#[from] ClassTypeParseError),
    #[error("Race parse error {0}")]
    RaceParseError(#[from] RaceParseError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct CharInfo {
    race: Race,
    clan: Clan,
    gender: Gender,
}

/// Takes a Document and a search expression, and will return
/// a `SearchError` if it is not found. Otherwise it will return
/// the found node.
macro_rules! ensure_node {
    ($doc:ident, $search:expr) => {{
        ensure_node!($doc, $search, 0)
    }};

    ($doc:ident, $search:expr, $nth:expr) => {{
        $doc.find($search)
            .nth($nth)
            .ok_or(SearchError::NodeNotFound(stringify!(
                $search, "(", $nth, ")"
            )))?
    }};
}

/// Holds data about the images for this character
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterImages {
    /// Small character avatar
    pub avatar_small: String,
    /// Full body image of the character
    pub full_body: String,
}

#[derive(Clone, Debug, Error)]
pub enum CharacterParseError {
    #[error("src was missing on node {node}")]
    UrlMissing { node: String },
    #[error("unable to find node {node} with an image")]
    NodeMissing { node: String },
}

impl CharacterImages {
    fn parse(doc: &Document) -> Result<Self, LodestoneError> {
        let face_url = ensure_node!(doc, Class("character-block__face"))
            .attr("src")
            .ok_or(CharacterParseError::UrlMissing {
                node: "character-block__face".into(),
            })?;
        let node = "js__image_popup".to_string();
        let body = doc
            .find(Class(node.as_str()))
            .next()
            .ok_or_else(|| CharacterParseError::NodeMissing { node: node.clone() })?
            .attr("href")
            .ok_or(CharacterParseError::UrlMissing { node })?;
        Ok(Self {
            avatar_small: face_url.to_string(),
            full_body: body.to_string(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SecondaryAttribute {
    MP(u32),
    GP(u32),
    CP(u32),
}

/// Holds all the data for a profile retrieved via Lodestone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Profile {
    /// The id associated with the profile
    pub user_id: u32,
    /// The profile's associated title
    pub title: Option<String>,
    /// The profile's associated Free Company
    pub free_company: Option<String>,
    /// The character's in-game name.
    pub name: String,
    /// The character's nameday
    pub nameday: String,
    /// The character's guardian
    pub guardian: String,
    /// The character's city state
    pub city_state: String,
    /// Which server the character is in.
    pub server: Server,
    /// What race the character is.
    pub race: Race,
    /// One of the two clans associated with their race.
    pub clan: Clan,
    /// Character's gender.
    pub gender: Gender,
    /// Max HP.
    pub hp: u32,
    /// Max MP.
    pub mp_or_gp: SecondaryAttribute,
    /// A list of attributes and their values.
    pub attributes: Attributes,
    /// A list of classes and their corresponding levels.
    classes: Classes,
    /// Collection of character images
    pub character_images: CharacterImages,
}

impl Profile {
    /// Gets a profile for a user given their lodestone user id.
    ///
    /// If you don't have the id, it is possible to use a
    /// `SearchBuilder` in order to find their profile directly.
    #[cfg(blocking)]
    pub fn get(user_id: u32) -> Result<Self, Error> {
        let main_doc = load_url(user_id, None)?;
        let classes_doc = load_url(user_id, Some("class_job"))?;

        //  Holds the string for Race, Clan, and Gender in that order
        Profile::parse_profile(user_id, &main_doc, &classes_doc)
    }

    pub async fn get_async(client: &reqwest::Client, user_id: u32) -> Result<Self, LodestoneError> {
        let class_page = load_profile_url_async(client, user_id, Some("class_job")).await?;
        let profile_page = load_profile_url_async(client, user_id, None).await?;
        let main_doc = Document::from(profile_page.as_str());
        let classes_doc = Document::from(class_page.as_str());

        //  Holds the string for Race, Clan, and Gender in that order
        Profile::parse_profile(user_id, &main_doc, &classes_doc)
    }

    fn parse_profile(
        user_id: u32,
        main_doc: &Document,
        classes_doc: &Document,
    ) -> Result<Profile, LodestoneError> {
        let char_info = Self::parse_char_info(main_doc)?;
        let (hp, mp) = Self::parse_char_param(main_doc)?;
        let value = Self {
            user_id,
            title: Self::parse_title(main_doc),
            free_company: Self::parse_free_company(main_doc),
            name: Self::parse_name(main_doc)?,
            nameday: Self::parse_nameday(main_doc)?,
            guardian: Self::parse_guardian(main_doc)?,
            city_state: Self::parse_city_state(main_doc)?,
            server: Self::parse_server(main_doc)?,
            race: char_info.race,
            clan: char_info.clan,
            gender: char_info.gender,
            hp,
            mp_or_gp: mp,
            attributes: Self::parse_attributes(main_doc)?,
            classes: Self::parse_classes(classes_doc)?,
            character_images: CharacterImages::parse(main_doc)?,
        };
        Ok(value)
    }

    /// Get the level of a specific class for this profile.
    ///
    /// This can be used to query whether or not a job is unlocked.
    /// For instance if Gladiator is below 30, then Paladin will
    /// return None. If Paladin is unlocked, both Gladiator and
    /// Paladin will return the same level.
    pub fn level(&self, class: ClassType) -> Option<u32> {
        self.class_info(class).map(|v| v.level)
    }

    /// Gets this profile's data for a given class
    pub fn class_info(&self, class: ClassType) -> Option<ClassInfo> {
        self.classes.get(class)
    }

    /// Borrows the full map of classes, e.g. for iteration in calling code
    pub fn all_class_info(&self) -> &Classes {
        &self.classes
    }

    fn parse_free_company(doc: &Document) -> Option<String> {
        doc.find(Class("character__freecompany__name"))
            .next()
            .and_then(|n| n.find(Name("a")).next().map(|n| n.text()))
    }

    fn parse_title(doc: &Document) -> Option<String> {
        doc.find(Class("frame__chara__title"))
            .next()
            .map(|node| node.text())
    }

    fn parse_name(doc: &Document) -> Result<String, SearchError> {
        Ok(ensure_node!(doc, Class("frame__chara__name")).text())
    }

    fn parse_nameday(doc: &Document) -> Result<String, SearchError> {
        Ok(ensure_node!(doc, Class("character-block__birth")).text())
    }

    fn parse_guardian(doc: &Document) -> Result<String, SearchError> {
        Ok(ensure_node!(doc, Class("character-block__name"), 1).text())
    }

    fn parse_city_state(doc: &Document) -> Result<String, SearchError> {
        Ok(ensure_node!(doc, Class("character-block__name"), 2).text())
    }

    fn parse_server(doc: &Document) -> Result<Server, SearchError> {
        let text = ensure_node!(doc, Class("frame__chara__world")).text();
        let server = text
            .split('\u{A0}')
            .next()
            .ok_or(SearchError::InvalidData("Could not find server string."))?;
        // Servers now show as Server Name [Datacenter]
        Ok(Server::from_str(server.split(' ').next().ok_or(
            SearchError::InvalidData("Server string was empty"),
        )?)?)
    }

    fn parse_char_info(doc: &Document) -> Result<CharInfo, SearchError> {
        let char_block = {
            let mut block = ensure_node!(doc, Class("character-block__name")).inner_html();
            block = block.replace(' ', "_");
            block = block.replace("<br>", " ");
            block.replace("_/_", " ")
        };

        let char_info = char_block
            .split_whitespace()
            .map(|e| e.replace('_', " "))
            .collect::<Vec<String>>();

        println!("{:?}", char_info);
        if !(char_info.len() == 3 || char_info.len() == 4) {
            return Err(SearchError::InvalidData("character block name"));
        }

        //  If the length is 4, then the race is "Au Ra"
        if char_info.len() == 4 {
            Ok(CharInfo {
                race: Race::Aura,
                clan: Clan::from_str(&char_info[2])?,
                gender: Gender::from_str(&char_info[3])?,
            })
        } else {
            Ok(CharInfo {
                race: Race::from_str(&char_info[0])?,
                clan: Clan::from_str(&char_info[1])?,
                gender: Gender::from_str(&char_info[2])?,
            })
        }
    }

    fn parse_char_param(doc: &Document) -> Result<(u32, SecondaryAttribute), SearchError> {
        let attr_block = ensure_node!(doc, Class("character__param"));
        let mut hp = None;
        let mut secondary_attribute = None;
        for item in attr_block.find(Name("li")) {
            if item
                .find(Class("character__param__text__hp--en-us"))
                .count()
                == 1
            {
                hp = Some(ensure_node!(item, Name("span")).text().parse::<u32>()?);
            } else if item
                .find(Class("character__param__text__mp--en-us"))
                .count()
                == 1
            {
                secondary_attribute = Some(SecondaryAttribute::MP(
                    ensure_node!(item, Name("span")).text().parse::<u32>()?,
                ));
            } else if item
                .find(Class("character__param__text__gp--en-us"))
                .count()
                == 1
            {
                secondary_attribute = Some(SecondaryAttribute::GP(
                    ensure_node!(item, Name("span")).text().parse::<u32>()?,
                ));
            } else if item
                .find(Class("character__param__text__mp--en-us"))
                .count()
                == 1
            {
                secondary_attribute = Some(SecondaryAttribute::CP(
                    ensure_node!(item, Name("span")).text().parse::<u32>()?,
                ));
            } else {
                continue;
            }
        }

        Ok((
            hp.ok_or(SearchError::NodeNotFound("HP not found"))?,
            secondary_attribute.ok_or(SearchError::InvalidData("MP or GP not found"))?,
        ))
    }

    fn parse_attributes(doc: &Document) -> Result<Attributes, SearchError> {
        let block = ensure_node!(doc, Class("character__profile__data"));
        let mut attributes = Attributes::new();
        for item in block.find(Name("tr")) {
            let name = ensure_node!(item, Name("span")).text();
            let value = Attribute {
                level: ensure_node!(item, Name("td")).text().parse::<u16>()?,
            };
            attributes.insert(name, value);
        }
        Ok(attributes)
    }

    fn parse_classes(doc: &Document) -> Result<Classes, SearchError> {
        let mut classes = Classes::new();

        for list in doc.find(Class("character__content")).take(4) {
            for item in list.find(Name("li")) {
                let name = ensure_node!(item, Class("character__job__name")).text();
                let classinfo = match ensure_node!(item, Class("character__job__level"))
                    .text()
                    .as_str()
                {
                    "-" => None,
                    level => {
                        let text = ensure_node!(item, Class("character__job__exp")).text();
                        let mut parts = text.split(" / ");
                        let current_xp = parts
                            .next()
                            .ok_or(SearchError::InvalidData("character__job__exp"))?;
                        let max_xp = parts
                            .next()
                            .ok_or(SearchError::InvalidData("character__job__exp"))?;
                        Some(ClassInfo {
                            level: level.parse()?,
                            current_xp: match current_xp {
                                "--" => None,
                                value => Some(value.replace(',', "").parse()?),
                            },
                            max_xp: match max_xp {
                                "--" => None,
                                value => Some(value.replace(',', "").parse()?),
                            },
                        })
                    }
                };

                //  For classes that have multiple titles (e.g., Paladin / Gladiator), grab the first one.
                let name = name
                    .split(" / ")
                    .next()
                    .ok_or(SearchError::InvalidData("character__job__name"))?;

                let class = ClassType::from_str(name)?;

                //  If the class added was a secondary job, then associated that level
                //  with its lower level counterpart as well. This makes returning the
                //  level for a particular grouping easier at the cost of memory.
                match class {
                    ClassType::Paladin => classes.insert(ClassType::Gladiator, classinfo),
                    ClassType::Warrior => classes.insert(ClassType::Marauder, classinfo),
                    ClassType::WhiteMage => classes.insert(ClassType::Conjurer, classinfo),
                    ClassType::Monk => classes.insert(ClassType::Pugilist, classinfo),
                    ClassType::Dragoon => classes.insert(ClassType::Lancer, classinfo),
                    ClassType::Ninja => classes.insert(ClassType::Rogue, classinfo),
                    ClassType::Bard => classes.insert(ClassType::Archer, classinfo),
                    ClassType::BlackMage => classes.insert(ClassType::Thaumaturge, classinfo),
                    ClassType::Summoner => classes.insert(ClassType::Arcanist, classinfo),
                    _ => (),
                }

                classes.insert(class, classinfo);
            }
        }

        Ok(classes)
    }
}
