use std::str::FromStr;
use thiserror::Error;

#[derive(Clone, Debug, Error)]
#[error("Invalid grand company string '{0}'")]
pub struct GrandCompanyParseError(String);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum GrandCompany {
    Maelstrom,
    TwinAdder,
    ImmortalFlames,
    Unaffiliated,
}

impl FromStr for GrandCompany {
    type Err = GrandCompanyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match &*s.to_uppercase() {
            "MAELSTROM" => Ok(GrandCompany::Maelstrom),
            "ORDER OF THE TWIN ADDER" | "TWIN ADDER" => Ok(GrandCompany::TwinAdder),
            "IMMORTAL FLAMES" => Ok(GrandCompany::ImmortalFlames),
            "" | "NONE" | "UNAFFILIATED" => Ok(GrandCompany::Unaffiliated),
            x => Err(GrandCompanyParseError(x.into())),
        }
    }
}
