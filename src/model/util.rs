use crate::LodestoneError;
#[cfg(blocking)]
use crate::CLIENT;
use reqwest::StatusCode;

/// The URL base for profiles.
static BASE_PROFILE_URL: &str = "https://na.finalfantasyxiv.com/lodestone/character/";

pub(crate) async fn load_profile_url_async(
    client: &reqwest::Client,
    user_id: u32,
    subpage: Option<&str>,
) -> Result<String, LodestoneError> {
    let subpage = match subpage {
        None => "".to_string(),
        Some(v) => format!("{}/", v),
    };
    let response = client
        .get(&format!("{}{}/{}", BASE_PROFILE_URL, user_id, subpage))
        .send()
        .await?;
    let status_code = response.status().as_u16();
    if status_code == 404 {
        return Err(LodestoneError::CharacterNotFound(user_id));
    }
    let text = response.text().await?;
    Ok(text)
}

#[cfg(blocking)]
pub(crate) fn load_url(user_id: u32, subpage: Option<&str>) -> Result<Document, Error> {
    let subpage = match subpage {
        None => "".to_string(),
        Some(v) => format!("{}/", v),
    };
    let mut response = CLIENT
        .get(&format!("{}{}/{}", BASE_PROFILE_URL, user_id, subpage))
        .send()?;
    let status_code = response.status().as_u16();
    if status_code == 404 {
        return Err(LodestoneError::CharacterNotFound(user_id));
    }
    let text = response.text()?;
    Ok(Document::from(text.as_str()))
}
