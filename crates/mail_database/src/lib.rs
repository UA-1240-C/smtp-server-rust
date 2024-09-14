pub mod models;
pub mod schema;

use diesel::prelude::*;
use diesel::pg::PgConnection;
use models::NewUser;
use thiserror::Error;
use argon2::{Argon2, PasswordHasher, PasswordVerifier, Params, Algorithm, Version};
use argon2::password_hash::{rand_core, PasswordHash, SaltString};

// Define custom error type for mail database
#[derive(Error, Debug)]
pub enum MailError {
    #[error("Database connection error")]
    ConnectionError(#[from] diesel::result::ConnectionError),

    #[error("Connection is None")]
    NoConnection,

    #[error("Query error")]
    QueryError(#[from] diesel::result::Error),
    
    #[error("User not found")]
    UserNotFound,

    #[error("User already exist")]
    UserAlreadyExist,

    #[error("User authentication failed")]
    UserAuthError,

    #[error("User is not logged in failed")]
    UserNotLoggedIn,

    #[error("Password hashing error")]
    PasswordHashError,

    #[error("Password verification error")]
    PasswordVerifyError,
}

pub trait IMailDB {
    fn connect(&mut self, connection_string: &str) -> Result<(), MailError>;
    fn disconnect(&mut self);
    fn is_connected(&mut self) -> bool;
    fn sign_up(&mut self, user_name: &str, password: &str) -> Result<(), MailError>;
    fn login(&mut self, user_name: &str, password: &str) -> Result<(), MailError>;
    fn insert_email(&mut self, receiver: &str, subject: &str, body: &str) -> Result<(), MailError>;
    fn insert_multiple_emails(&mut self, receivers: Vec<&str>, subject: &str, body: &str) -> Result<(), MailError>;
    fn user_exists(&mut self, user_name: &str) -> Result<bool,MailError>;
}

// PostgreSQL MailDB implementation using Diesel
#[derive(Default)]
pub struct PgMailDB<'a> {
    host_name: String,
    host_id: u32,
    user_name: Option<String>,
    user_id: Option<u32>,
    conn: Option<PgConnection>,
    hash_algorithm : Argon2<'a>,
}

impl<'a> PgMailDB<'a> {
    pub fn new(host_name: String) -> Self {
        let argon2 = Argon2::new(Algorithm::Argon2id,
            Version::V0x13,
            Params::new(65536, 2, 1, None).unwrap()
        );
        PgMailDB {
            host_name: host_name,
            hash_algorithm: argon2,
            ..Default::default()
        }
    }

    fn ensure_host_id(&mut self) -> Result<(), MailError> {
        use crate::schema::hosts::dsl::*;

        let conn = self.conn.as_mut().ok_or_else(|| MailError::NoConnection)?;
        // Check if the host exists
        let existing_host = hosts
            .filter(host_name.eq(&self.host_name))
            .select(host_id)
            .first::<i32>(conn)
            .ok();

        if let Some(id) = existing_host {
            self.host_id = id as u32;
            return Ok(());
        }

        // Insert the new host and get its ID
        self.host_id = diesel::insert_into(hosts)
            .values(host_name.eq(&self.host_name))
            .returning(host_id)
            .get_result::<i32>(conn)? 
            as u32;

        Ok(())
    }
}

impl<'a> IMailDB for PgMailDB<'a> {
    fn connect(&mut self, connection_string: &str) -> Result<(), MailError> {        
        self.conn = Some(PgConnection::establish(connection_string)?);
        
        self.ensure_host_id()?;

        Ok(())

    }

    fn disconnect(&mut self) {
        self.conn = None;
    }

    fn is_connected(&mut self) -> bool {
        if let Some(ref mut conn) = self.conn {
            let result = diesel::sql_query("SELECT 1").execute(conn);
            return result.is_ok();
        }
        false
    }

    fn sign_up(&mut self, input_user_name: &str, password: &str) -> Result<(), MailError> {
        use crate::schema::users::dsl::*;

        let conn = self.conn.as_mut().ok_or_else(|| MailError::NoConnection)?;

        // Check if the user exists
        let existing_user = users.filter(user_name.eq(input_user_name))
            .filter(host_id.eq(self.host_id as i32))
            .select(user_id)
            .first::<i32>(conn)
            .ok();

        if let Some(_) = existing_user {
            return Err(MailError::UserAlreadyExist);
        }
        // Generate hashed password
        let salt = SaltString::generate(&mut rand_core::OsRng);
        let hashed_password = self.hash_algorithm.hash_password(password.as_bytes(), &salt)
            .map_err(|_| MailError::PasswordHashError)?
            .to_string();

        // Add new user
        let new_user = NewUser {
            host_id: self.host_id as i32, 
            user_name: input_user_name, 
            password_hash: &hashed_password
        };
        diesel::insert_into(users)
            .values(&new_user)
            .execute(conn)?;

        Ok(())
    }

    fn login(&mut self, input_user_name: &str, password: &str) -> Result<(), MailError> {
        use crate::schema::users::dsl::*;
        use crate::models::UserInfo;

        let conn = self.conn.as_mut().ok_or_else(|| MailError::NoConnection)?;

        // Check if the user exists
        let user_info = users
            .filter(user_name.eq(input_user_name))
            .filter(host_id.eq(self.host_id as i32))
            .select(UserInfo::as_select())
            .first::<UserInfo>(conn)
            .map_err(|_| MailError::UserNotFound)?;

        let parsed_hash = PasswordHash::new(&user_info.password_hash)
            .map_err(|_| MailError::PasswordVerifyError)?;

        // Verify password
        if self.hash_algorithm.verify_password(password.as_bytes(), &parsed_hash).is_ok() {
            self.user_id = Some(user_info.user_id as u32);
            self.user_name = Some(user_info.user_name);
            Ok(())
        } else {
            Err(MailError::UserAuthError)
        }
    }

    fn insert_email(&mut self, receiver: &str, subject: &str, body: &str) -> Result<(), MailError> {
        self.insert_multiple_emails(vec![receiver], subject, body)
    }

    fn insert_multiple_emails(&mut self, receivers: Vec<&str>, subject: &str, body: &str) -> Result<(), MailError> {
        if self.user_id.is_none() || self.user_name.is_none() {
            return Err(MailError::UserNotLoggedIn);
        }

        use crate::schema::users::dsl::*;
        use crate::schema::mailBodies::dsl::*;
        use crate::schema::emailMessages;
        use crate::models::NewMail;

        self.conn.as_mut().ok_or_else(|| MailError::NoConnection)?
            .transaction(
            |connection|
            {
                let mut receiver_ids: Vec<i32> = Vec::new();

                for receiver in receivers {
                    let receiver_id: i32 = users.filter(user_name.eq(receiver))
                        .filter(host_id.eq(self.host_id as i32))
                        .select(user_id)
                        .first::<i32>(connection)?;

                    receiver_ids.push(receiver_id);
                }

                let body_id: i32 =  diesel::insert_into(mailBodies)
                    .values(body_content.eq(body))
                    .returning(mail_body_id)
                    .get_result(connection)?;

                for id in receiver_ids {
                    let new_mail = NewMail {
                        sender_id: self.user_id.unwrap() as i32,
                        recipient_id: id,
                        subject : subject,
                        mail_body_id : body_id,
                        is_received: false
                    };
                    diesel::insert_into(emailMessages::table)
                        .values(new_mail)
                        .execute(connection)?;
                }
                diesel::result::QueryResult::Ok(())
            }
        )?;
        Ok(())
    }

    fn user_exists(&mut self, input_user_name: &str) -> Result<bool,MailError> {
        use crate::schema::users::dsl::*;

        let conn = self.conn.as_mut().ok_or_else(|| MailError::NoConnection)?;

        Ok(users.filter(user_name.eq(input_user_name))
            .filter(host_id.eq(self.host_id as i32))
            .select(host_id)
            .first::<i32>(conn)
            .is_ok()
        )

    }
}
