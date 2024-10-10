use diesel::prelude::*;
use chrono::NaiveDateTime;


#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::hosts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Host {
    pub host_id: i32,
    pub host_name: String,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserInfo {
    pub user_id: i32,
    pub user_name: String,
    pub password_hash: String,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub user_id: i32,
    pub host_id: i32,
    pub user_name: String,
    pub password_hash: String,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::users)]
pub struct NewUser<'a> {
    pub host_id: i32,
    pub user_name: &'a str,
    pub password_hash: &'a str,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::email_messages)]
pub struct NewMail<'a> {
    pub sender_id: i32,
    pub recipient_id: i32,
    pub subject: &'a str,
    pub mail_body_id: i32,
    pub is_received: bool,
}
