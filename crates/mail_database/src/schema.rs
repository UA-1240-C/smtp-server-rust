// @generated automatically by Diesel CLI.

diesel::table! {
    #[sql_name = "emailMessages"]
    email_messages (email_message_id) {
        email_message_id -> Int4,
        sender_id -> Nullable<Int4>,
        recipient_id -> Nullable<Int4>,
        #[max_length = 255]
        subject -> Nullable<Varchar>,
        mail_body_id -> Nullable<Int4>,
        sent_at -> Nullable<Timestamp>,
        is_received -> Nullable<Bool>,
    }
}

diesel::table! {
    hosts (host_id) {
        host_id -> Int4,
        #[max_length = 100]
        host_name -> Varchar,
    }
}

diesel::table! {
    #[sql_name = "mailBodies"]
    mail_bodies (mail_body_id) {
        mail_body_id -> Int4,
        body_content -> Text,
    }
}

diesel::table! {
    users (user_id) {
        user_id -> Int4,
        host_id -> Int4,
        #[max_length = 100]
        user_name -> Varchar,
        password_hash -> Text,
        created_at -> Timestamp,
    }
}

diesel::joinable!(email_messages -> mail_bodies (mail_body_id));
diesel::joinable!(users -> hosts (host_id));

diesel::allow_tables_to_appear_in_same_query!(
    email_messages,
    hosts,
    mail_bodies,
    users,
);
