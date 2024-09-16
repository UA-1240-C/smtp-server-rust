use std::{fmt::Debug, slice::SliceIndex};
mod commands; use commands::*;
use logger_proc_macro::*;

#[allow(non_camel_case_types)]
#[derive(Eq, Debug, PartialEq)]
pub enum RequestType {
    EHLO(String),
    STARTTLS,
    AUTH_PLAIN(String),
    REGISTER(String),
    MAIL_FROM(String),
    RCPT_TO(String),
    DATA,
    QUIT,
    HELP,
    NOOP,
    RSET,
}

impl std::fmt::Display for RequestType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RequestType::EHLO(_) => write!(f, "{EHLO}"),
            RequestType::STARTTLS => write!(f, "{STARTTLS}"),
            RequestType::AUTH_PLAIN(_) => write!(f, "{AUTH_PLAIN}"),
            RequestType::REGISTER(_) => write!(f, "{REGISTER}"),
            RequestType::MAIL_FROM(_) => write!(f, "{MAIL_FROM}"),
            RequestType::RCPT_TO(_) => write!(f, "{RCPT_TO}"),
            RequestType::DATA => write!(f, "{DATA}"),
            RequestType::QUIT => write!(f, "{QUIT}"),
            RequestType::HELP => write!(f, "{HELP}"),
            RequestType::NOOP => write!(f, "{NOOP}"),
            RequestType::RSET => write!(f, "{RSET}"),

        }
    }

}

impl RequestType {
    #[log(trace)]
    pub fn parse(raw_request: &str) -> Result<RequestType, String> {
        let raw_request = raw_request.trim_start().trim_end();
        let request_res: Result<RequestType, String>;

        if raw_request.starts_with(EHLO) || raw_request.starts_with(HELO) {
            request_res = RequestType::parse_command_with_arg(RequestType::EHLO, raw_request, EHLO.len() + 1..);
        } else if raw_request.starts_with(STARTTLS) {
            request_res = Ok(RequestType::STARTTLS);
        } else if raw_request.starts_with(AUTH_PLAIN) {
            request_res =  RequestType::parse_command_with_arg(RequestType::AUTH_PLAIN, raw_request, AUTH_PLAIN.len() + 1..);
        } else if raw_request.starts_with(REGISTER) {
            request_res =  RequestType::parse_command_with_arg(RequestType::REGISTER, raw_request, REGISTER.len() + 1..);
        } else if raw_request.starts_with(MAIL_FROM) {
            request_res =  RequestType::parse_command_with_arg(RequestType::MAIL_FROM, raw_request, MAIL_FROM.len() + 3..raw_request.len() - 1);
        } else if raw_request.starts_with(RCPT_TO) {
            request_res =  RequestType::parse_command_with_arg(RequestType::RCPT_TO, raw_request, RCPT_TO.len() + 3..raw_request.len() - 1);
        } else if raw_request.starts_with(DATA) {
            request_res = Ok(RequestType::DATA);
        } else if raw_request.starts_with(QUIT) {
            request_res = Ok(RequestType::QUIT);
        } else if raw_request.starts_with(HELP) {
            request_res = Ok(RequestType::HELP);
        } else if raw_request.starts_with(NOOP) {
            request_res = Ok(RequestType::NOOP);
        } else if raw_request.starts_with(RSET) {
            request_res = Ok(RequestType::RSET);
        } else {
            request_res = Err("Could not parse the SMTP command".into());
        }

        request_res
    }
    #[log(trace)]
    fn parse_command_with_arg<I: SliceIndex<str> + Debug>(cmd_type: fn(String) -> RequestType, raw_request: &str, slice: I) -> Result<RequestType, String> 
    where
        <I as SliceIndex<str>>::Output: std::fmt::Display + Debug,
    {
        let argument = raw_request.get(slice);
        if let Some(argument) = argument {
            Ok(cmd_type(argument.to_string()))
        } else {
            RequestType::argument_parsing_error(&cmd_type(String::new()).to_string())
        }
    }

    fn argument_parsing_error(command: &str) -> Result<RequestType, String> {
        Err(format!("Could not parse the argument for the command: {}", command))
    }

}




#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_ehlo() {
        let request = RequestType::parse("EHLO example.com").unwrap();
        assert_eq!(request, RequestType::EHLO("example.com".to_string()));
    }

    #[test]
    fn test_parse_helo() {
        let request = RequestType::parse("HELO example.com").unwrap();
        assert_eq!(request, RequestType::EHLO("example.com".to_string()));
    }

    #[test]
    fn test_parse_helo_err() {
        // HELO and EHLO must have the argument part
        let request = RequestType::parse("HELO");
        assert!(request.is_err());
    }

    #[test]
    fn test_parse_starttls() {
        let request = RequestType::parse("STARTTLS").unwrap();
        assert_eq!(request, RequestType::STARTTLS);
    }

    #[test]
    fn test_parse_auth_plain() {
        let request = RequestType::parse("AUTH PLAIN login_and_password").unwrap();
        assert_eq!(request, RequestType::AUTH_PLAIN("login_and_password".to_string()));

    }

    #[test]
    fn test_parse_register() {
        let request = RequestType::parse("REGISTER login_and_password").unwrap();
        assert_eq!(request, RequestType::REGISTER("login_and_password".to_string()));
    }

    #[test]
    fn test_parse_mail_from() {
        let request = RequestType::parse("MAIL FROM:<user@example.com>").unwrap();
        assert_eq!(request, RequestType::MAIL_FROM("user@example.com".to_string()));
    }

    #[test]
    fn test_broken_mail_from() {
        let request = RequestType::parse("MAIL FROM:<");
        assert_eq!(request.is_err(), true);
    }

    #[test]
    fn test_parse_rcpt_to() {
        let request = RequestType::parse("RCPT TO:<user@example.com>").unwrap();
        assert_eq!(request, RequestType::RCPT_TO("user@example.com".to_string()));
    }

    #[test]
    fn test_parse_data() {
        let request = RequestType::parse("DATA").unwrap();
        assert_eq!(request, RequestType::DATA);
    }

    #[test]
    fn test_parse_quit() {
        let request = RequestType::parse("QUIT").unwrap();
        assert_eq!(request, RequestType::QUIT);
    }

    #[test]
    fn test_parse_help() {
        let request = RequestType::parse("HELP").unwrap();
        assert_eq!(request, RequestType::HELP);
    }

    #[test]
    fn test_parse_noop() {
        let request = RequestType::parse("NOOP").unwrap();
        assert_eq!(request, RequestType::NOOP);
    }

    #[test]
    fn test_parse_rset() {
        let request = RequestType::parse("RSET").unwrap();
        assert_eq!(request, RequestType::RSET);
    }

    #[test]
    fn test_parse_unexpected() {
        let request = RequestType::parse("RCV FROM:<user@example.com>");
        assert_eq!(request.is_err(), true);
    }
}

 
