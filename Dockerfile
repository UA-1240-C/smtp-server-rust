FROM rust:1.81.0

WORKDIR /usr/src/smtp_server
COPY . .

RUN cargo build --bin server --release 

EXPOSE 2525

CMD ["./target/release/server"]
