mod utils;

#[cfg(test)]
mod tests {
    use super::*;
    use utils::*;
    use mail_database::IMailDB;
    use diesel::prelude::*;

    static CONNECTION_STR : &str = "postgres://postgres:password@127.0.0.1:5432";

    #[test]
    fn connection_test() {
        let (mut ctx, _) = setup_database(CONNECTION_STR, "connection_test");

        let conn_str = ctx.get_connection_string();
        let pg = &mut ctx.pg_db;

        assert!(pg.connect(&conn_str).is_ok());
        assert!(pg.connect("fake_connection_string").is_err());
    }

    #[test]
    fn is_connected_test() {
        let (mut ctx, _) = setup_database(CONNECTION_STR, "is_connected_test");

        let conn_str = ctx.get_connection_string();
        let pg = &mut ctx.pg_db;

        assert!(!pg.is_connected());
        assert!(pg.connect(&conn_str).is_ok());
        assert!(pg.is_connected());
        pg.disconnect();
        assert!(!pg.is_connected());
    }

    #[test]
    fn sign_up_test() {
        let (mut ctx, mut conn) = setup_database(CONNECTION_STR, "sign_up_test");

        let conn_str = ctx.get_connection_string();
        let pg = &mut ctx.pg_db;

        use mail_database::schema::users::dsl::*;
        use mail_database::models::UserInfo;

        let user_names = vec!["user1", "user2","user3"];

        assert!(pg.connect(&conn_str).is_ok());
        for u in &user_names {
            assert!(pg.sign_up(u, "password").is_ok());
        }

        let user_info = users
                .filter(user_name.like("user%"))
                .select(UserInfo::as_select())
                .load::<UserInfo>(&mut conn)
                .unwrap();
        for i in 0..user_names.len() {
            assert_eq!(user_names[i], user_info[i].user_name);
        }    

        assert!(pg.sign_up("user1", "password").is_err());
        pg.disconnect();
        assert!(pg.sign_up("user4", "password").is_err());
    }

    #[test]
    fn login_test() {
        let (mut ctx, _) = setup_database(CONNECTION_STR, "login_test");

        let conn_str = ctx.get_connection_string();
        let pg = &mut ctx.pg_db;

        assert!(pg.connect(&conn_str).is_ok());
        assert!(pg.login("user1", "password").is_err());
        assert!(pg.sign_up("user1", "password").is_ok());
        assert!(pg.login("user1", "password").is_ok());
        assert!(pg.login("user1", "fake_password").is_err());

        pg.disconnect();
        assert!(pg.login("user1", "password").is_err());
    }

    #[test]
    fn insert_emails_test() {
        use mail_database::schema::mail_bodies::dsl::*;
        use mail_database::schema::email_messages;

        let (mut ctx, mut conn) = setup_database(CONNECTION_STR, "insert_emails_test");

        let conn_str = ctx.get_connection_string();
        let pg = &mut ctx.pg_db;

        assert!(pg.connect(&conn_str).is_ok());
        assert!(pg.sign_up("user1", "password").is_ok());
        assert!(pg.insert_multiple_emails(vec!["user1", "user2"], "subj", "body").is_err());

        assert!(pg.login("user1", "password").is_ok());
        assert!(pg.insert_multiple_emails(vec!["user1", "not-existing-user2"], "subj", "body").is_err());
        assert!(pg.sign_up("user2", "password").is_ok());
        assert!(pg.insert_multiple_emails(vec!["user1", "user2"], "subj", "body").is_ok());

        let bodies_count = mail_bodies.count().get_result::<i64>(&mut conn).unwrap();
        assert_eq!(bodies_count, 1);
        let mails_count = email_messages::table.count().get_result::<i64>(&mut conn).unwrap();
        assert_eq!(mails_count, 2);

        assert!(pg.insert_email("user2", "subj", "body").is_ok());
        let bodies_count = mail_bodies.count().get_result::<i64>(&mut conn).unwrap();
        assert_eq!(bodies_count, 2);
        let mails_count = email_messages::table.count().get_result::<i64>(&mut conn).unwrap();
        assert_eq!(mails_count, 3);

        pg.disconnect();
        assert!(pg.insert_multiple_emails(vec!["user1"], "subj", "body").is_err());
    }

}


