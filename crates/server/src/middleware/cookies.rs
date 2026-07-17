use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SameSite {
    Lax,
    Strict,
}

pub struct CookieOptions<'a> {
    pub name: &'a str,
    pub secure: bool,
    pub same_site: SameSite,
    pub max_age_seconds: Option<i64>,
}

pub fn read_cookie<'a>(header: &'a str, expected_name: &str) -> Option<&'a str> {
    header.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        (name == expected_name && valid_cookie_value(value)).then_some(value)
    })
}

pub fn set_cookie(value: &str, options: &CookieOptions<'_>) -> String {
    let mut cookie = format!("{}={value}; Path=/; HttpOnly", options.name);
    if options.secure {
        cookie.push_str("; Secure");
    }
    let same_site = match options.same_site {
        SameSite::Lax => "Lax",
        SameSite::Strict => "Strict",
    };
    let _ = write!(cookie, "; SameSite={same_site}");
    if let Some(seconds) = options.max_age_seconds {
        let _ = write!(cookie, "; Max-Age={seconds}");
    }
    cookie
}

pub fn clear_cookie(options: &CookieOptions<'_>) -> String {
    let expired = CookieOptions {
        name: options.name,
        secure: options.secure,
        same_site: options.same_site,
        max_age_seconds: Some(0),
    };
    set_cookie("", &expired)
}

fn valid_cookie_value(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}
