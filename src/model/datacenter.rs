use std::fmt;
use std::str::FromStr;

#[derive(Clone, Debug, thiserror::Error)]
#[error("Invalid datacenter string '{0}'")]
pub struct DatacenterParseError(String);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Datacenter {
    Aether,
    Chaos,
    Crystal,
    Dynamis,
    Elemental,
    Gaia,
    Light,
    Mana,
    Primal,
    Materia,
    Meteor,
}

/// Case insensitive FromStr impl for datacenters.
impl FromStr for Datacenter {
    type Err = DatacenterParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match &*s.to_uppercase() {
            "AETHER" => Ok(Datacenter::Aether),
            "CHAOS" => Ok(Datacenter::Chaos),
            "CRYSTAL" => Ok(Datacenter::Crystal),
            "ELEMENTAL" => Ok(Datacenter::Elemental),
            "GAIA" => Ok(Datacenter::Gaia),
            "LIGHT" => Ok(Datacenter::Light),
            "MANA" => Ok(Datacenter::Mana),
            "PRIMAL" => Ok(Datacenter::Primal),
            "MATERIA" => Ok(Datacenter::Materia),
            "METEOR" => Ok(Datacenter::Meteor),
            "DYNAMIS" => Ok(Datacenter::Dynamis),
            x => Err(DatacenterParseError(x.into())),
        }
    }
}

impl fmt::Display for Datacenter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let datacenter = match *self {
            Datacenter::Aether => "Aether",
            Datacenter::Chaos => "Chaos",
            Datacenter::Crystal => "Crystal",
            Datacenter::Elemental => "Elemental",
            Datacenter::Gaia => "Gaia",
            Datacenter::Light => "Light",
            Datacenter::Mana => "Mana",
            Datacenter::Primal => "Primal",
            Datacenter::Materia => "Materia",
            Datacenter::Meteor => "Meteor",
            Datacenter::Dynamis => "Dynamis",
        };

        write!(f, "{}", datacenter)
    }
}
