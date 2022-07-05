pub mod model;
pub mod search;

use crate::model::profile::{CharacterParseError, SearchError};
use crate::model::server::ServerParseError;
use crate::model::standings::{FreeCompanyLeaderboardError, FreeCompanyParseError};
use thiserror::Error;

// Lazy static client to avoid creating new ones every time
#[cfg(blocking)]
lazy_static::lazy_static! {
    static ref CLIENT: reqwest::blocking::Client = reqwest::blocking::Client::new();
}

#[derive(Debug, Error)]
pub enum LodestoneError {
    #[error("Search error {0}")]
    SearchError(#[from] SearchError),
    #[error("http error {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Error parsing server {0}")]
    ServerParserError(#[from] ServerParseError),
    #[error("Leaderboard error {0}")]
    LeaderboardError(#[from] FreeCompanyLeaderboardError),
    #[error("Freecompany parse error {0}")]
    FreecompanyParseError(#[from] FreeCompanyParseError),
    #[error("IO Error {0}")]
    IOError(#[from] std::io::Error),
    #[error("Error parsing character data {0}")]
    CharacterParseError(#[from] CharacterParseError),
}

#[cfg(test)]
mod tests {
    use crate::model::profile::{Profile, SecondaryAttribute};

    #[tokio::test]
    async fn can_grab_profile() {
        use crate::model::profile::Profile;
        let client = reqwest::Client::new();
        Profile::get_async(&client, 11908971).await.unwrap();
        Profile::get_async(&client, 38686892).await.unwrap();
    }

    #[tokio::test]
    async fn can_create_search() {
        use crate::model::datacenter::Datacenter;
        use crate::model::gc::GrandCompany;
        use crate::model::language::Language;
        use crate::search::SearchBuilder;

        let profiles = SearchBuilder::new()
            .character("Strawberry Custard")
            .datacenter(Datacenter::Primal)
            .lang(Language::English)
            .grand_company(GrandCompany::Maelstrom)
            .send_async(&reqwest::Client::new())
            .await
            .unwrap();

        assert_eq!(profiles.len(), 1);
    }

    #[tokio::test]
    async fn profile_is_correct() {
        use crate::model::{
            clan::Clan, class::ClassType, datacenter::Datacenter, gc::GrandCompany, gender::Gender,
            language::Language, race::Race,
        };
        use crate::search::SearchBuilder;
        let test_client = reqwest::Client::new();
        let profiles = SearchBuilder::new()
            .character("Strawberry Custard")
            .datacenter(Datacenter::Primal)
            .lang(Language::English)
            .grand_company(GrandCompany::Maelstrom)
            .send_async(&test_client)
            .await
            .unwrap();

        assert_eq!(profiles.len(), 1);

        let strawberry = profiles.get(0).unwrap();
        assert_eq!(strawberry.name, "Strawberry Custard");
        assert_eq!(strawberry.world, "Famfrit [Primal]");

        let strawberry = Profile::get_async(&test_client, strawberry.user_id)
            .await
            .unwrap();
        assert_eq!(strawberry.name, "Strawberry Custard");
        assert_eq!(strawberry.nameday, "3rd Sun of the 1st Umbral Moon");
        assert_eq!(strawberry.guardian, "Halone, the Fury");
        assert_eq!(strawberry.city_state, "Limsa Lominsa");

        assert_eq!(strawberry.race, Race::Lalafell);
        assert_eq!(strawberry.clan, Clan::Plainsfolk);
        assert_eq!(strawberry.gender, Gender::Female);
        assert_eq!(strawberry.level(ClassType::BlackMage), Some(70));
        // These stats were assumed to be correct after endwalker stat squish
        assert_eq!(strawberry.hp, 15141);
        assert_eq!(strawberry.mp_or_gp, SecondaryAttribute::MP(10000));

        let attribs = &strawberry.attributes;

        assert_eq!(attribs.get("Strength").unwrap().level, 130);
        assert_eq!(attribs.get("Dexterity").unwrap().level, 295);
        assert_eq!(attribs.get("Vitality").unwrap().level, 1246);
        assert_eq!(attribs.get("Intelligence").unwrap().level, 1349);
        assert_eq!(attribs.get("Mind").unwrap().level, 219);

        assert_eq!(attribs.get("Critical Hit Rate").unwrap().level, 1039);
        assert_eq!(attribs.get("Determination").unwrap().level, 617);
        assert_eq!(attribs.get("Direct Hit Rate").unwrap().level, 726);

        assert_eq!(attribs.get("Defense").unwrap().level, 933);
        assert_eq!(attribs.get("Magic Defense").unwrap().level, 1629);

        assert_eq!(attribs.get("Attack Power").unwrap().level, 130);
        assert_eq!(attribs.get("Skill Speed").unwrap().level, 364);

        assert_eq!(attribs.get("Attack Magic Potency").unwrap().level, 1349);
        assert_eq!(attribs.get("Healing Magic Potency").unwrap().level, 219);
        assert_eq!(attribs.get("Spell Speed").unwrap().level, 955);

        assert_eq!(attribs.get("Tenacity").unwrap().level, 364);
        assert_eq!(attribs.get("Piety").unwrap().level, 292);

        assert_eq!(attribs.get("Invalid Attribute"), None);

        let classes = &strawberry.all_class_info();

        assert_eq!(classes.get(ClassType::Paladin), None);
        let mut class = classes.get(ClassType::Gladiator).unwrap();
        assert_eq!(class.level, 22);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(30600));
        class = classes.get(ClassType::Warrior).unwrap();
        assert_eq!(class.level, 60);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(1387000));
        class = classes.get(ClassType::Marauder).unwrap();
        assert_eq!(class.level, 60);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(1387000));
        class = classes.get(ClassType::DarkKnight).unwrap();
        assert_eq!(class.level, 30);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(68400));
        assert_eq!(classes.get(ClassType::Gunbreaker), None);
        assert_eq!(classes.get(ClassType::WhiteMage), None);
        class = classes.get(ClassType::Conjurer).unwrap();
        assert_eq!(class.level, 6);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(1940));
        class = classes.get(ClassType::Scholar).unwrap();
        assert_eq!(class.level, 33);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(88700));
        class = classes.get(ClassType::Astrologian).unwrap();
        assert_eq!(class.level, 30);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(68400));
        assert_eq!(classes.get(ClassType::Monk), None);
        class = classes.get(ClassType::Pugilist).unwrap();
        assert_eq!(class.level, 15);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(13100));
        assert_eq!(classes.get(ClassType::Dragoon), None);
        class = classes.get(ClassType::Lancer).unwrap();
        assert_eq!(class.level, 17);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(17400));
        assert_eq!(classes.get(ClassType::Ninja), None);
        class = classes.get(ClassType::Rogue).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Samurai).unwrap();
        assert_eq!(class.level, 50);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(421000));
        assert_eq!(classes.get(ClassType::Bard), None);
        class = classes.get(ClassType::Archer).unwrap();
        assert_eq!(class.level, 16);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(15200));
        class = classes.get(ClassType::Machinist).unwrap();
        assert_eq!(class.level, 30);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(68400));
        assert_eq!(classes.get(ClassType::Dancer), None);
        class = classes.get(ClassType::BlackMage).unwrap();
        assert_eq!(class.level, 70);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(2923000));
        class = classes.get(ClassType::Thaumaturge).unwrap();
        assert_eq!(class.level, 70);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(2923000));
        class = classes.get(ClassType::Summoner).unwrap();
        assert_eq!(class.level, 33);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(88700));
        class = classes.get(ClassType::Arcanist).unwrap();
        assert_eq!(class.level, 33);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(88700));
        class = classes.get(ClassType::RedMage).unwrap();
        assert_eq!(class.level, 50);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(421000));
        assert_eq!(classes.get(ClassType::BlueMage), None);
        class = classes.get(ClassType::Carpenter).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Blacksmith).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Armorer).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Goldsmith).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Leatherworker).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Weaver).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Alchemist).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Culinarian).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Miner).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Botanist).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Fisher).unwrap();
        assert_eq!(class.level, 30);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(68400));
    }
}

#[cfg(test)]
#[cfg(blocking)]
mod tests {
    #[test]
    fn can_grab_profile() {
        use crate::model::profile::Profile;

        assert!(Profile::get(11908971).is_ok());
    }

    #[test]
    fn can_create_search() {
        use crate::model::datacenter::Datacenter;
        use crate::model::gc::GrandCompany;
        use crate::model::language::Language;
        use crate::search::SearchBuilder;

        let profiles = SearchBuilder::new()
            .character("Strawberry Custard")
            .datacenter(Datacenter::Primal)
            .lang(Language::English)
            .grand_company(GrandCompany::Maelstrom)
            .send()
            .unwrap();

        assert_eq!(profiles.len(), 1);
    }

    #[test]
    fn profile_is_correct() {
        use crate::model::{
            clan::Clan, class::ClassType, datacenter::Datacenter, gc::GrandCompany, gender::Gender,
            language::Language, race::Race,
        };
        use crate::search::SearchBuilder;

        let profiles = SearchBuilder::new()
            .character("Strawberry Custard")
            .datacenter(Datacenter::Primal)
            .lang(Language::English)
            .grand_company(GrandCompany::Maelstrom)
            .send()
            .unwrap();

        assert_eq!(profiles.len(), 1);

        let strawberry = profiles.get(0).unwrap();

        assert_eq!(strawberry.name, "Strawberry Custard");
        assert_eq!(strawberry.nameday, "3rd Sun of the 1st Umbral Moon");
        assert_eq!(strawberry.guardian, "Halone, the Fury");
        assert_eq!(strawberry.city_state, "Limsa Lominsa");

        assert_eq!(strawberry.race, Race::Lalafell);
        assert_eq!(strawberry.clan, Clan::Plainsfolk);
        assert_eq!(strawberry.gender, Gender::Female);
        assert_eq!(strawberry.level(ClassType::BlackMage), Some(70));

        assert_eq!(strawberry.hp, 45835);
        assert_eq!(strawberry.mp, 10000);

        let attribs = &strawberry.attributes;

        assert_eq!(attribs.get("Strength").unwrap().level, 130);
        assert_eq!(attribs.get("Dexterity").unwrap().level, 295);
        assert_eq!(attribs.get("Vitality").unwrap().level, 2937);
        assert_eq!(attribs.get("Intelligence").unwrap().level, 3031);
        assert_eq!(attribs.get("Mind").unwrap().level, 219);

        assert_eq!(attribs.get("Critical Hit Rate").unwrap().level, 2050);
        assert_eq!(attribs.get("Determination").unwrap().level, 1014);
        assert_eq!(attribs.get("Direct Hit Rate").unwrap().level, 1183);

        assert_eq!(attribs.get("Defense").unwrap().level, 2238);
        assert_eq!(attribs.get("Magic Defense").unwrap().level, 3912);

        assert_eq!(attribs.get("Attack Power").unwrap().level, 130);
        assert_eq!(attribs.get("Skill Speed").unwrap().level, 364);

        assert_eq!(attribs.get("Attack Magic Potency").unwrap().level, 3031);
        assert_eq!(attribs.get("Healing Magic Potency").unwrap().level, 219);
        assert_eq!(attribs.get("Spell Speed").unwrap().level, 1856);

        assert_eq!(attribs.get("Tenacity").unwrap().level, 364);
        assert_eq!(attribs.get("Piety").unwrap().level, 292);

        assert_eq!(attribs.get("Invalid Attribute"), None);

        let classes = &strawberry.all_class_info();

        assert_eq!(classes.get(ClassType::Paladin), None);
        let mut class = classes.get(ClassType::Gladiator).unwrap();
        assert_eq!(class.level, 22);
        assert_eq!(class.current_xp, Some(10122));
        assert_eq!(class.max_xp, Some(71400));
        class = classes.get(ClassType::Warrior).unwrap();
        assert_eq!(class.level, 60);
        assert_eq!(class.current_xp, Some(51841));
        assert_eq!(class.max_xp, Some(4470000));
        class = classes.get(ClassType::Marauder).unwrap();
        assert_eq!(class.level, 60);
        assert_eq!(class.current_xp, Some(51841));
        assert_eq!(class.max_xp, Some(4470000));
        class = classes.get(ClassType::DarkKnight).unwrap();
        assert_eq!(class.level, 30);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(162500));
        assert_eq!(classes.get(ClassType::Gunbreaker), None);
        assert_eq!(classes.get(ClassType::WhiteMage), None);
        class = classes.get(ClassType::Conjurer).unwrap();
        assert_eq!(class.level, 6);
        assert_eq!(class.current_xp, Some(1652));
        assert_eq!(class.max_xp, Some(4200));
        class = classes.get(ClassType::Scholar).unwrap();
        assert_eq!(class.level, 33);
        assert_eq!(class.current_xp, Some(173825));
        assert_eq!(class.max_xp, Some(203500));
        class = classes.get(ClassType::Astrologian).unwrap();
        assert_eq!(class.level, 30);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(162500));
        assert_eq!(classes.get(ClassType::Monk), None);
        class = classes.get(ClassType::Pugilist).unwrap();
        assert_eq!(class.level, 15);
        assert_eq!(class.current_xp, Some(7563));
        assert_eq!(class.max_xp, Some(30500));
        assert_eq!(classes.get(ClassType::Dragoon), None);
        class = classes.get(ClassType::Lancer).unwrap();
        assert_eq!(class.level, 17);
        assert_eq!(class.current_xp, Some(18054));
        assert_eq!(class.max_xp, Some(40500));
        assert_eq!(classes.get(ClassType::Ninja), None);
        class = classes.get(ClassType::Rogue).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Samurai).unwrap();
        assert_eq!(class.level, 50);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(864000));
        assert_eq!(classes.get(ClassType::Bard), None);
        class = classes.get(ClassType::Archer).unwrap();
        assert_eq!(class.level, 16);
        assert_eq!(class.current_xp, Some(32589));
        assert_eq!(class.max_xp, Some(35400));
        class = classes.get(ClassType::Machinist).unwrap();
        assert_eq!(class.level, 30);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(162500));
        assert_eq!(classes.get(ClassType::Dancer), None);
        class = classes.get(ClassType::BlackMage).unwrap();
        assert_eq!(class.level, 70);
        assert_eq!(class.current_xp, Some(6910613));
        assert_eq!(class.max_xp, Some(12449000));
        class = classes.get(ClassType::Thaumaturge).unwrap();
        assert_eq!(class.level, 70);
        assert_eq!(class.current_xp, Some(6910613));
        assert_eq!(class.max_xp, Some(12449000));
        class = classes.get(ClassType::Summoner).unwrap();
        assert_eq!(class.level, 33);
        assert_eq!(class.current_xp, Some(173825));
        assert_eq!(class.max_xp, Some(203500));
        class = classes.get(ClassType::Arcanist).unwrap();
        assert_eq!(class.level, 33);
        assert_eq!(class.current_xp, Some(173825));
        assert_eq!(class.max_xp, Some(203500));
        class = classes.get(ClassType::RedMage).unwrap();
        assert_eq!(class.level, 50);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(864000));
        assert_eq!(classes.get(ClassType::BlueMage), None);
        class = classes.get(ClassType::Carpenter).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Blacksmith).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Armorer).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Goldsmith).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Leatherworker).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Weaver).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Alchemist).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Culinarian).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Miner).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Botanist).unwrap();
        assert_eq!(class.level, 1);
        assert_eq!(class.current_xp, Some(0));
        assert_eq!(class.max_xp, Some(300));
        class = classes.get(ClassType::Fisher).unwrap();
        assert_eq!(class.level, 30);
        assert_eq!(class.current_xp, Some(47790));
        assert_eq!(class.max_xp, Some(162500));
    }
}
