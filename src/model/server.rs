use crate::model::datacenter::{Datacenter, DatacenterParseError};
use crate::model::server::ServerCategory::{Congested, New, Preferred, Standard};
use select::document::Document;
use select::node::Node;
use select::predicate::Class;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use crate::LodestoneError;

static SERVER_STATUS_URL: &'static str = "https://na.finalfantasyxiv.com/lodestone/worldstatus/";

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum CharacterAvailability {
    CharactersAvailable,
    CharactersUnavailable,
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum ServerParseError {
    #[error("node was missing: {}", node)]
    NodeMissing { node: String },
    #[error("invalid server status, found {}", actual)]
    CategoryParseError { actual: String },
    #[error("{0}")]
    DatacenterParseError(#[from] DatacenterParseError)
}

impl Display for CharacterAvailability {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            CharacterAvailability::CharactersAvailable => write!(f, "Characters available"),
            CharacterAvailability::CharactersUnavailable => write!(f, "Characters not available"),
        }
    }
}

impl CharacterAvailability {
    fn parse_from(node: &Node) -> Result<Self, ServerParseError> {
        node.find(Class("world-ic__available"))
            .next()
            .ok_or(ServerParseError::NodeMissing {
                node: "world-ic__available".to_string(),
            })
            .map(|_| Self::CharactersAvailable)
            .or(node
                .find(Class("world-ic__unavailable"))
                .next()
                .ok_or(ServerParseError::NodeMissing {
                    node: "world-ic__unavailable".to_string(),
                })
                .map(|_| Self::CharactersUnavailable))
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ServerStatus {
    Online(ServerCategory, CharacterAvailability),
    PartialMaintenance(ServerCategory, CharacterAvailability),
    Maintenance,
}

impl ServerStatus {
    fn parse_from(node: &Node) -> Result<ServerStatus, ServerParseError> {
        node.find(Class("world-ic__1"))
            .next()
            .ok_or(ServerParseError::NodeMissing {
                node: "world-ic__1".to_string(),
            })
            .map(|_| Ok(ServerStatus::Online(ServerCategory::parse_from(node)?, CharacterAvailability::parse_from(node)?)))
            .or(node
                .find(Class("world-ic__2"))
                .next()
                .ok_or(ServerParseError::NodeMissing {
                    node: "world-ic__2".to_string(),
                })
                .map(|_| Ok(ServerStatus::PartialMaintenance(ServerCategory::parse_from(node)?, CharacterAvailability::parse_from(node)?))))
            .or(node
                .find(Class("world-ic__3"))
                .next()
                .ok_or(ServerParseError::NodeMissing {
                    node: "world-ic__3".to_string(),
                })
                .map(|_| Ok(ServerStatus::Maintenance)))?

    }
}

impl Display for ServerStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            // only write the outer state
            ServerStatus::Online(_, _) => write!(f, "Online"),
            ServerStatus::PartialMaintenance(_, _) => write!(f, "Partial Maintenance"),
            ServerStatus::Maintenance => write!(f, "Maintenance"),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ServerCategory {
    Standard,
    Preferred,
    Congested,
    New
}

impl ServerCategory {
    fn parse_from(n: &Node) -> Result<Self, ServerParseError> {
        let node_text = n
            .find(Class("world-list__world_category"))
            .next()
            .ok_or(ServerParseError::NodeMissing {
                node: "world-list__world_category".to_string(),
            })?
            .text();
        Ok(node_text.parse::<ServerCategory>()?)
    }
}

impl Display for ServerCategory {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Standard => write!(f, "Standard"),
            Preferred => write!(f, "Preferred"),
            Congested => write!(f, "Conjested"),
            New => write!(f, "New"),
        }
    }
}

impl FromStr for ServerCategory {
    type Err = ServerParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        match trimmed {
            "Standard" => Ok(Standard),
            "Preferred" => Ok(Preferred),
            "Congested" => Ok(Congested),
            "New" => Ok(New),
            _ => Err(ServerParseError::CategoryParseError {
                actual: trimmed.to_string(),
            }),
        }
    }
}

/// Gets current server status info detailing whether the server is online, or if character creation is limited
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct ServerDetails {
    pub name: String,
    pub status: ServerStatus
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct DataCenterDetails {
    pub name: Datacenter,
    pub servers: Vec<ServerDetails>,
}

impl DataCenterDetails {
    /// Downloads the status of all servers including the character availability and preferred status.
    pub async fn send_async(client: &reqwest::Client) -> Result<Vec<Self>, LodestoneError> {
        let value = client.get(SERVER_STATUS_URL).send().await?.text().await?;
        let document = Document::from(value.as_str());
        Ok(Self::parse_from_doc(&document)?)
    }

    /// *Blocking version*
    /// Requires feature - `blocking`
    /// Downloads the status of all servers including the character availability and preferred status.
    #[cfg(blocking)]
    pub fn send() -> Result<Vec<Self>, Error> {
        let value = client.get(SERVER_STATUS_URL).send().text();
        let document = Document::from(value.as_str());
        Ok(Self::parse_from_doc(document))
    }

    fn parse_from_doc(doc: &Document) -> Result<Vec<Self>, ServerParseError> {
        doc.find(Class("world-dcgroup__item"))
            .map(|dc| {
                let name = dc
                    .find(Class("world-dcgroup__header"))
                    .next()
                    .ok_or_else(|| ServerParseError::NodeMissing {
                        node: "world-dcgroup__header missing".to_string(),
                    })?
                    .text()
                    .trim()
                    .parse()?;
                Ok(Self {
                    name,
                    servers: ServerDetails::parse_from_doc(&dc)?,
                })
            })
            .collect()
    }
}

impl ServerDetails {
    fn parse_from_doc(doc: &Node) -> Result<Vec<Self>, ServerParseError> {
        doc.find(Class("world-list__item"))
            .map(|n| {
                let status = ServerStatus::parse_from(&n)?;

                let name = n
                    .find(Class("world-list__world_name"))
                    .next()
                    .ok_or(ServerParseError::NodeMissing {
                        node: "world-list__world_name".to_string(),
                    })?
                    .text()
                    .trim()
                    .to_string();

                Ok(ServerDetails {
                    name,
                    status
                })
            })
            .collect()
    }
}

/// An enumeration for the servers that are currently available.
/// This list is taken from https://na.finalfantasyxiv.com/lodestone/worldstatus/
/// and the order should be identical.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Server {
    //  Elemental
    Aegis,
    Atomos,
    Carbuncle,
    Garuda,
    Gungnir,
    Kujata,
    Ramuh,
    Tonberry,
    Typhon,
    Unicorn,
    //  Gaia
    Alexander,
    Bahamut,
    Durandal,
    Fenrir,
    Ifrit,
    Ridill,
    Tiamat,
    Ultima,
    Valefor,
    Yojimbo,
    Zeromus,
    //  Mana
    Anima,
    Asura,
    Belias,
    Chocobo,
    Hades,
    Ixion,
    Mandragora,
    Masamune,
    Pandaemonium,
    Shinryu,
    Titan,
    //  Aether
    Adamantoise,
    Cactuar,
    Faerie,
    Gilgamesh,
    Jenova,
    Midgardsormr,
    Sargatanas,
    Siren,
    //  Primal
    Behemoth,
    Excalibur,
    Exodus,
    Famfrit,
    Hyperion,
    Lamia,
    Leviathan,
    Ultros,
    //  Crystal
    Balmung,
    Brynhildr,
    Coeurl,
    Diabolos,
    Goblin,
    Malboro,
    Mateus,
    Zalera,
    //  Chaos
    Cerberus,
    Louisoix,
    Moogle,
    Omega,
    Ragnarok,
    Spriggan,
    //  Light
    Lich,
    Odin,
    Phoenix,
    Shiva,
    Twintania,
    Zodiark,
    // Oceania
    Bismarck,
    Ravana,
    Sephirot,
    Sophia,
    Zurvan,
}

/// Case insensitive FromStr impl for servers.
impl FromStr for Server {
    type Err = ServerParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match &*s.to_uppercase() {
            //  Elemental
            "AEGIS" => Ok(Server::Aegis),
            "ATOMOS" => Ok(Server::Atomos),
            "CARBUNCLE" => Ok(Server::Carbuncle),
            "GARUDA" => Ok(Server::Garuda),
            "GUNGNIR" => Ok(Server::Gungnir),
            "KUJATA" => Ok(Server::Kujata),
            "RAMUH" => Ok(Server::Ramuh),
            "TONBERRY" => Ok(Server::Tonberry),
            "TYPHON" => Ok(Server::Typhon),
            "UNICORN" => Ok(Server::Unicorn),
            //  Gaia
            "ALEXANDER" => Ok(Server::Alexander),
            "BAHAMUT" => Ok(Server::Bahamut),
            "DURANDAL" => Ok(Server::Durandal),
            "FENRIR" => Ok(Server::Fenrir),
            "IFRIT" => Ok(Server::Ifrit),
            "RIDILL" => Ok(Server::Ridill),
            "TIAMAT" => Ok(Server::Tiamat),
            "ULTIMA" => Ok(Server::Ultima),
            "VALEFOR" => Ok(Server::Valefor),
            "YOJIMBO" => Ok(Server::Yojimbo),
            "ZEROMUS" => Ok(Server::Zeromus),
            //  Mana
            "ANIMA" | "ANIUMA" => Ok(Server::Anima),
            "ASURA" => Ok(Server::Asura),
            "BELIAS" => Ok(Server::Belias),
            "CHOCOBO" => Ok(Server::Chocobo),
            "HADES" => Ok(Server::Hades),
            "IXION" => Ok(Server::Ixion),
            "MANDRAGORA" => Ok(Server::Mandragora),
            "MASAMUNE" => Ok(Server::Masamune),
            "PANDAEMONIUM" => Ok(Server::Pandaemonium),
            "SHINRYU" => Ok(Server::Shinryu),
            "TITAN" => Ok(Server::Titan),
            //  Aether
            "ADAMANTOISE" => Ok(Server::Adamantoise),
            "BALMUNG" => Ok(Server::Balmung),
            "CACTUAR" => Ok(Server::Cactuar),
            "COEURL" => Ok(Server::Coeurl),
            "FAERIE" => Ok(Server::Faerie),
            "GILGAMESH" => Ok(Server::Gilgamesh),
            "GOBLIN" => Ok(Server::Goblin),
            "JENOVA" => Ok(Server::Jenova),
            "MATEUS" => Ok(Server::Mateus),
            "MIDGARDSORMR" => Ok(Server::Midgardsormr),
            "SARGATANAS" => Ok(Server::Sargatanas),
            "SIREN" => Ok(Server::Siren),
            "ZALERA" => Ok(Server::Zalera),
            //  Primal
            "BEHEMOTH" => Ok(Server::Behemoth),
            "BRYNHILDR" => Ok(Server::Brynhildr),
            "DIABOLOS" => Ok(Server::Diabolos),
            "EXCALIBUR" => Ok(Server::Excalibur),
            "EXODUS" => Ok(Server::Exodus),
            "FAMFRIT" => Ok(Server::Famfrit),
            "HYPERION" => Ok(Server::Hyperion),
            "LAMIA" => Ok(Server::Lamia),
            "LEVIATHAN" => Ok(Server::Leviathan),
            "MALBORO" => Ok(Server::Malboro),
            "ULTROS" => Ok(Server::Ultros),
            //  Chaos
            "CERBERUS" => Ok(Server::Cerberus),
            "LOUISOIX" => Ok(Server::Louisoix),
            "MOOGLE" => Ok(Server::Moogle),
            "OMEGA" => Ok(Server::Omega),
            "RAGNAROK" => Ok(Server::Ragnarok),
            "SPRIGGAN" => Ok(Server::Spriggan),
            //  Light
            "LICH" => Ok(Server::Lich),
            "ODIN" => Ok(Server::Odin),
            "PHOENIX" => Ok(Server::Phoenix),
            "SHIVA" => Ok(Server::Shiva),
            "TWINTANIA" => Ok(Server::Twintania),
            "ZODIARK" => Ok(Server::Zodiark),
            // Materia
            "BISMARCK" => Ok(Server::Bismarck),
            "RAVANA" => Ok(Server::Ravana),
            "SEPHIROT" => Ok(Server::Sephirot),
            "SOPHIA" => Ok(Server::Sophia),
            "ZURVAN" => Ok(Server::Zurvan),
            x => Err(ServerParseError::CategoryParseError { actual: x.into() }),
        }
    }
}

impl fmt::Display for Server {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let server = match *self {
            //  Elemental
            Server::Aegis => "Aegis",
            Server::Atomos => "Atomos",
            Server::Carbuncle => "Carbuncle",
            Server::Garuda => "Garuda",
            Server::Gungnir => "Gungnir",
            Server::Kujata => "Kujata",
            Server::Ramuh => "Ramuh",
            Server::Tonberry => "Tonberry",
            Server::Typhon => "Typhon",
            Server::Unicorn => "Unicorn",
            //  Gaia
            Server::Alexander => "Alexander",
            Server::Bahamut => "Bahamut",
            Server::Durandal => "Durandal",
            Server::Fenrir => "Fenrir",
            Server::Ifrit => "Ifrit",
            Server::Ridill => "Ridill",
            Server::Tiamat => "Tiamat",
            Server::Ultima => "Ultima",
            Server::Valefor => "Valefor",
            Server::Yojimbo => "Yojimbo",
            Server::Zeromus => "Zeromus",
            //  Mana
            Server::Anima => "Aniuma",
            Server::Asura => "Asura",
            Server::Belias => "Belias",
            Server::Chocobo => "Chocobo",
            Server::Hades => "Hades",
            Server::Ixion => "Ixion",
            Server::Mandragora => "Mandragora",
            Server::Masamune => "Masamune",
            Server::Pandaemonium => "Pandaemonium",
            Server::Shinryu => "Shinryu",
            Server::Titan => "Titan",
            //  Aether
            Server::Adamantoise => "Adamantoise",
            Server::Balmung => "Balmung",
            Server::Cactuar => "Cactuar",
            Server::Coeurl => "Coeurl",
            Server::Faerie => "Faerie",
            Server::Gilgamesh => "Gilgamesh",
            Server::Goblin => "Goblin",
            Server::Jenova => "Jenova",
            Server::Mateus => "Mateus",
            Server::Midgardsormr => "Midgardsormr",
            Server::Sargatanas => "Sargatanas",
            Server::Siren => "Siren",
            Server::Zalera => "Zalera",
            //  Primal
            Server::Behemoth => "Behemoth",
            Server::Brynhildr => "Brynhildr",
            Server::Diabolos => "Diabolos",
            Server::Excalibur => "Excalibur",
            Server::Exodus => "Exodus",
            Server::Famfrit => "Famfrit",
            Server::Hyperion => "Hyperion",
            Server::Lamia => "Lamia",
            Server::Leviathan => "Leviathan",
            Server::Malboro => "Malboro",
            Server::Ultros => "Ultros",
            //  Chaos
            Server::Cerberus => "Cerberus",
            Server::Louisoix => "Louisoix",
            Server::Moogle => "Moogle",
            Server::Omega => "Omega",
            Server::Ragnarok => "Ragnarok",
            Server::Spriggan => "Spriggan",
            //  Light
            Server::Lich => "Lich",
            Server::Odin => "Odin",
            Server::Phoenix => "Phoenix",
            Server::Shiva => "Shiva",
            Server::Twintania => "Twintania",
            Server::Zodiark => "Zodiark",
            Server::Bismarck => "Bismarck",
            Server::Ravana => "Ravana",
            Server::Sephirot => "Sephirot",
            Server::Sophia => "Sophia",
            Server::Zurvan => "Zurvan",
        };

        write!(f, "{}", server)
    }
}

#[cfg(test)]
mod test {
    use crate::model::datacenter::Datacenter;
    use crate::model::server::{DataCenterDetails, ServerStatus};
    use select::document::Document;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_status_parse_test() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("sample_data/");
        let mut normal_path = d.clone();
        normal_path.push("server_status.html");
        let mut bad_path = d.clone();
        bad_path.push("server_status_bad.html");
        let sample = fs::read_to_string(normal_path).unwrap();
        let document = Document::from(sample.as_str());
        let parsed_dc = DataCenterDetails::parse_from_doc(&document).unwrap();
        let known_dc = [
            Datacenter::Elemental,
            Datacenter::Gaia,
            Datacenter::Mana,
            Datacenter::Aether,
            Datacenter::Primal,
            Datacenter::Crystal,
            Datacenter::Chaos,
            Datacenter::Light,
        ];
        for (i, x) in parsed_dc.iter().enumerate() {
            assert_eq!(*known_dc.get(i).unwrap(), x.name);
        }
        let maintenance_mode = std::fs::read_to_string(bad_path).unwrap();
        let bad_servers = Document::from(maintenance_mode.as_str());
        let parsed_dc = DataCenterDetails::parse_from_doc(&bad_servers).unwrap();
        for (i, x) in parsed_dc.iter().enumerate() {
            assert_eq!(*known_dc.get(i).unwrap(), x.name);
            for dc in &x.servers {
                assert_eq!(dc.status, ServerStatus::Maintenance);
            }
        }
    }

    #[tokio::test]
    async fn test_network_parse() {
        let server = DataCenterDetails::send_async(&reqwest::Client::new())
            .await
            .unwrap();
        println!("{:?}", server);
        assert!(server.len() > 4);
    }
}
