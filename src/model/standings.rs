use crate::model::datacenter::{Datacenter, DatacenterParseError};
use crate::model::gc::{GrandCompany, GrandCompanyParseError};
use crate::model::server::{Server, ServerParseError};
use std::fmt::Write;
use std::io::Cursor;
use std::num::ParseIntError;
use thiserror::Error as ThisError;
use select::document::Document;
use select::node::Node;
use select::predicate::{Class, Element, Name, Predicate};
use crate::LodestoneError;
use crate::model::standings::FreeCompanyParseError::{CreditsMissing, DataCenterMissing, FreeCompanyMissing, GrandCompanyMissing, RankingMissing, WorldNameMissing};

#[derive(Debug)]
pub struct FreeCompanyLeaderboardQuery {
    /// Unknown value
    pub filter: Option<i8>,
    /// Server to filter by
    pub world_name: Option<Server>,
    /// Datacenter to filter by
    pub dc_group: Option<Datacenter>,
    // Ranged 1..=5
    pub page: Option<u8>,
    /// Grand company to search the leaderboard for
    /// represented as gcid in the query, 1 = maelstrom, 2 = twinadder, 3 = immortal flames, None = all
    pub grand_company: Option<GrandCompany>
}

/// Represents the ranking of a free company
pub struct FreeCompanyRankingResult {
    pub ranking: i32,
    pub free_company_name: String,
    pub world_name: Server,
    pub datacenter: Datacenter,
    pub grand_company: GrandCompany,
    // really not sure how big this number is max, i64 to be safe.
    pub company_credits: i64,
}

#[derive(Debug, ThisError)]
pub enum FreeCompanyParseError {
    #[error("Couldn't find the table")]
    TableNotFound,
    #[error("Ranking missing")]
    RankingMissing,
    #[error("Data center missing")]
    DataCenterMissing,
    #[error("World name missing")]
    WorldNameMissing,
    #[error("Grand company missing")]
    GrandCompanyMissing,
    #[error("Credits missing")]
    CreditsMissing,
    #[error("Free company missing")]
    FreeCompanyMissing,
    #[error("Parse int error {0}")]
    ParseIntError(#[from] ParseIntError),
    #[error("Server parse error {0}")]
    ServerParseError(#[from] ServerParseError),
    #[error("Free company error {0}")]
    DatacenterParseError(#[from] DatacenterParseError),
    #[error("Free company error {0}")]
    GrandCompanyParseError(#[from] GrandCompanyParseError)
}

#[derive(Debug, ThisError)]
pub enum FreeCompanyLeaderboardError {
    #[error("{0}")]
    FreeCompanyParseError(#[from] FreeCompanyParseError),
    #[error("{0}")]
    IOError(#[from] std::io::Error)
}

impl FreeCompanyLeaderboardQuery {
    const LEADERBOARD: &'static str = "https://na.finalfantasyxiv.com/lodestone/ranking/fc/";

    fn get_query_parts(&self) -> String {
        let mut s = String::new();
        {
            let str = &mut s;
            if let Some(f) = self.filter {
                let _ = write!(str, "filter={}&", f);
            }
            if let Some(world_name) = self.world_name {
                let _ = write!(str, "world_name={}&", world_name);
            }
            if let Some(d) = self.dc_group {
                let _ = write!(str, "dcgroup={}&", d);
            }
            if let Some(p) = self.page {
                let _ = write!(str, "page={}&", p);
            }
        }
        s
    }

    fn parse_node(row: &Node) -> Result<FreeCompanyRankingResult, FreeCompanyParseError> {
        let mut children = row.children().filter(|e| Element.matches(e));

        let ranking = children.next().ok_or(RankingMissing)?.text().trim().parse()?;
        let _ = children.next(); // crest
        let free_company_data = children.next().ok_or(FreeCompanyMissing)?;
        // h4 = fc name, p = Server [Datacenter]
        let mut fc_data_children = free_company_data.children().filter(|e| Element.matches(e));
        let free_company_name = fc_data_children.next().ok_or(FreeCompanyMissing)?.text();
        let server_str = fc_data_children.next().ok_or(WorldNameMissing)?.text();
        let mut server_str = server_str.split(' ');
        let world_name = server_str.next().ok_or(WorldNameMissing)?.trim().parse()?;
        // dc text should be [Datacenter], remove []'s so it can be parsed
        let datacenter = server_str.next().ok_or(DataCenterMissing)?;
        let datacenter = datacenter[1..datacenter.len() - 1].parse()?;
        let grand_company = children.next().ok_or(GrandCompanyMissing)?.find(Element).next().ok_or(GrandCompanyMissing)?.attr("alt").ok_or(GrandCompanyMissing)?.parse()?;
        let company_credits = children.next().ok_or(CreditsMissing)?.text().trim().parse()?;
        Ok(FreeCompanyRankingResult {
            ranking,
            free_company_name,
            world_name,
            datacenter,
            grand_company,
            company_credits
        })
    }

    fn parse_data(document: &Document) -> Result<Vec<FreeCompanyRankingResult>, FreeCompanyParseError> {

        if let Some(table) = document.find(Class("ranking-character")).next() {
            table.find(Name("tr")).map(|row| {
                Self::parse_node(&row)
            })
                .collect()
        } else {
            Err(FreeCompanyParseError::TableNotFound)
        }
    }

    pub async fn weekly(&self, week: Option<i32>) -> Result<Vec<FreeCompanyRankingResult>, LodestoneError> {
        let week = week.map(|i| format!("/{i}")).unwrap_or_default();
        let response = reqwest::get(format!("{}weekly{week}?{}", Self::LEADERBOARD, self.get_query_parts())).await?;
        let document = Document::from_read(Cursor::new(response.bytes().await?))?;
        Ok(Self::parse_data(&document)?)
    }

    pub async fn monthly(&self, month: Option<i32>) -> Result<Vec<FreeCompanyRankingResult>, LodestoneError> {
        let month = month.map(|m| format!("/{m}")).unwrap_or_default();
        let response = reqwest::get(format!("{}monthly{month}?{}", Self::LEADERBOARD, self.get_query_parts())).await?;
        let document = Document::from_read(Cursor::new(response.bytes().await?))?;
        Ok(Self::parse_data(&document)?)
    }
}

#[cfg(test)]
mod test {
    use crate::model::standings::FreeCompanyLeaderboardQuery;

    #[tokio::test]
    async fn test_weekly_parse() {
        let query = FreeCompanyLeaderboardQuery {
            filter: None,
            world_name: None,
            dc_group: None,
            page: None,
            grand_company: None
        };

        let weekly = query.weekly(None).await.unwrap();
        assert!(!weekly.is_empty());
    }
}