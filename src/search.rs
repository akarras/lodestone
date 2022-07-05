use select::document::Document;
use select::predicate::Class;

use crate::model::datacenter::Datacenter;
use crate::model::gc::GrandCompany;
use crate::model::language::Language;
use crate::model::server::Server;
#[cfg(blocking)]
use crate::CLIENT;

use crate::LodestoneError;
use std::collections::HashSet;
use std::fmt::Write;

static BASE_SEARCH_URL: &str = "https://na.finalfantasyxiv.com/lodestone/character/?";

#[derive(Clone, Debug, Default)]
pub struct SearchBuilder {
    server: Option<Server>,
    datacenter: Option<Datacenter>,
    character: Option<String>,
    lang: HashSet<Language>,
    gc: HashSet<GrandCompany>,
}

/// Holds shallow data about a profile
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileSearchResult {
    pub user_id: u32,
    pub name: String,
    pub world: String,
}

impl SearchBuilder {
    pub fn new() -> Self {
        SearchBuilder {
            ..Default::default()
        }
    }

    /// Builds the search and executes it, returning a list of profiles
    /// that match the given criteria.
    #[cfg(blocking)]
    pub fn send(self) -> Result<Vec<ProfileSearchResult>, LodestoneError> {
        let url = self.build_url();

        let response = CLIENT.get(&url).send()?;
        let text = response.text()?;
        let doc = Document::from(text.as_str());

        Ok(SearchBuilder::parse_profile(doc))
    }

    pub async fn send_async(
        self,
        client: &reqwest::Client,
    ) -> Result<Vec<ProfileSearchResult>, LodestoneError> {
        let url = self.build_url();
        let response = client.get(&url).send().await?;
        let text = response.text().await?;
        let doc = Document::from(text.as_str());

        Ok(SearchBuilder::parse_profile(doc))
    }

    fn parse_profile(doc: Document) -> Vec<ProfileSearchResult> {
        doc.find(Class("entry__link"))
            .filter_map(|node| {
                let user_id = node.attr("href").and_then(|text| {
                    let digits = text
                        .chars()
                        .skip_while(|ch| !ch.is_ascii_digit())
                        .take_while(|ch| ch.is_ascii_digit())
                        .collect::<String>();

                    digits.parse::<u32>().ok()
                })?;
                let name = node.find(Class("entry__name")).map(|m| m.text()).next()?;
                let world = node.find(Class("entry__world")).map(|m| m.text()).next()?;
                Some(ProfileSearchResult {
                    user_id,
                    name,
                    world,
                })
            })
            .collect()
    }

    fn build_url(self) -> String {
        let mut url = BASE_SEARCH_URL.to_owned();

        if let Some(name) = self.character {
            let _ = write!(url, "q={}&", name);
        }

        if let Some(dc) = self.datacenter {
            let _ = write!(url, "worldname=_dc_{}&", dc);
        }

        if let Some(s) = self.server {
            let _ = write!(url, "worldname={}&", s);
        }

        self.lang.iter().for_each(|lang| {
            let _ = match lang {
                Language::Japanese => write!(url, "blog_lang=ja&"),
                Language::English => write!(url, "blog_lang=en&"),
                Language::German => write!(url, "blog_lang=de&"),
                Language::French => write!(url, "blog_lang=fr&"),
            };
        });

        self.gc.iter().for_each(|gc| {
            let _ = match gc {
                GrandCompany::Unaffiliated => write!(url, "gcid=0&"),
                GrandCompany::Maelstrom => write!(url, "gcid=1&"),
                GrandCompany::TwinAdder => write!(url, "gcid=2&"),
                GrandCompany::ImmortalFlames => write!(url, "gcid=3&"),
            };
        });

        let url = url.trim_end_matches('&').to_string();
        url
    }

    /// A character name to search for. This can only be called once,
    /// and any further calls will simply overwrite the previous name.
    pub fn character(mut self, name: &str) -> Self {
        self.character = Some(name.into());
        self
    }

    /// A datacenter to search in. Mutually exclusive to server.
    /// If a server was specified before calling this method,
    /// it will be replaced by the newer datacenter.
    pub fn datacenter<D: Into<Datacenter>>(mut self, datacenter: D) -> Self {
        self.datacenter = Some(datacenter.into());
        self.server = None;
        self
    }

    /// A server to search in. Mutually exclusive to datacenter.
    /// If a datacenter was specified before calling this method,
    /// it will be replaced by the newer server.
    pub fn server<S: Into<Server>>(mut self, server: S) -> Self {
        self.server = Some(server.into());
        self.datacenter = None;
        self
    }

    /// Which language to filter by.
    /// You can add multiple languages by calling this multiple times.
    pub fn lang<L: Into<Language>>(mut self, lang: L) -> Self {
        self.lang.insert(lang.into());
        self
    }

    /// Which grand company to filter by.
    /// You can add multiple grand company filters by calling this multiple times.
    pub fn grand_company<G: Into<GrandCompany>>(mut self, gc: G) -> Self {
        self.gc.insert(gc.into());
        self
    }
}
