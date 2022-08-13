use crate::error::Error;
use crate::data;

pub struct Session
{
    pub user: data::User,
}

pub fn getSession(data_source: &data::DataManager) -> Result<Session, Error>
{
    let user = if let Some(u) = data_source.findUser("MetroWind")?
    {
        u
    }
    else
    {
        data_source.createUser(data::User::new(0, "MetroWind".to_owned()))?
    };

    Ok(Session { user: user, })
}
