use crate::settings::models::DBSetting;
use crate::{settings, utils};
use diesel::prelude::*;
use diesel::MysqlConnection;

// Setting
pub fn add_new_setting(s: &DBSetting, conn: &MysqlConnection) -> Result<(), diesel::result::Error> {
    use crate::schema::settings::dsl::*;
    diesel::insert_into(settings)
        .values(s)
        .execute(conn)
        .unwrap();
    Ok(())
}

pub fn update_setting(s: &DBSetting, conn: &MysqlConnection) -> Result<(), diesel::result::Error> {
    use crate::schema::settings::dsl::*;

    let result1 = diesel::update(settings.filter(id.eq(s.id)))
        .set((
            value.eq(s.value.clone()),
            updated.eq(utils::get_current_time()),
        ))
        .execute(conn)?;
    if result1 == 0 {
        return add_new_setting(s, conn);
    }
    Ok(())
}

pub fn get_setting(
    k: &str,
    conn: &MysqlConnection,
) -> Result<Option<settings::models::DBSetting>, diesel::result::Error> {
    use crate::schema::settings::dsl::*;
    let found_user = settings
        .filter(setting.like(k.to_string()))
        .first::<DBSetting>(conn)
        .optional()?;
    Ok(found_user)
}

pub fn get_settings(
    conn: &MysqlConnection,
) -> Result<Vec<settings::models::DBSetting>, diesel::result::Error> {
    use crate::schema::settings::dsl::*;
    settings.load::<DBSetting>(conn)
}
