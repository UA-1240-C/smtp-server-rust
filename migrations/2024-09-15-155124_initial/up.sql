-- Your SQL goes here
CREATE TABLE hosts (
    host_id SERIAL PRIMARY KEY,
    host_name VARCHAR(100) UNIQUE NOT NULL
);

CREATE TABLE users (
    user_id SERIAL PRIMARY KEY,
	host_id INTEGER NOT NULL REFERENCES hosts(host_id),
    user_name VARCHAR(100) NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE "mailBodies" (
    mail_body_id SERIAL PRIMARY KEY,
    body_content TEXT NOT NULL
);

CREATE TABLE "emailMessages" (
    email_message_id SERIAL PRIMARY KEY,
    sender_id INTEGER REFERENCES users(user_id),
    recipient_id INTEGER REFERENCES users(user_id),
    subject VARCHAR(255),
	mail_body_id INTEGER REFERENCES "mailBodies"(mail_body_id),
    sent_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_received BOOLEAN DEFAULT FALSE
);