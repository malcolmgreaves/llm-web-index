use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::names)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Name {
    pub id: i32,
    pub name: String,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = crate::schema::names)]
pub struct NewName {
    pub name: String,
}
