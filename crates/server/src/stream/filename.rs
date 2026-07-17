use axum::http::{HeaderValue, header::InvalidHeaderValue};

pub const MAX_FILENAME_CHARS: usize = 120;
const DEFAULT_FILENAME: &str = "video.mp4";
const DEFAULT_EXTENSION: &str = ".mp4";
const SAFE_MEDIA_EXTENSIONS: [&str; 4] = ["mp4", "webm", "mkv", "mov"];
const HEX_DIGITS: &[u8; 16] = b"0123456789ABCDEF";
const HIGH_NIBBLE_SHIFT: u32 = 4;
const LOW_NIBBLE_MASK: u8 = 0x0f;

pub fn sanitize_filename(value: &str) -> String {
    let cleaned = value
        .chars()
        .take(MAX_FILENAME_CHARS)
        .map(sanitize_character)
        .collect::<String>();
    let without_parent_markers = remove_parent_markers(&cleaned);
    let trimmed = without_parent_markers.trim_matches([' ', '.', '_']);

    if trimmed.is_empty() {
        return DEFAULT_FILENAME.to_owned();
    }
    ensure_media_extension(trimmed)
}

pub fn attachment_content_disposition(filename: &str) -> Result<HeaderValue, InvalidHeaderValue> {
    let sanitized = sanitize_filename(filename);
    let fallback = ascii_fallback(&sanitized);
    let encoded = encode_extended_value(&sanitized);
    let value = format!("attachment; filename=\"{fallback}\"; filename*=UTF-8''{encoded}");

    HeaderValue::from_bytes(value.as_bytes())
}

fn sanitize_character(character: char) -> char {
    if character.is_alphanumeric() || matches!(character, ' ' | '-' | '_' | '.') {
        return character;
    }
    '_'
}

fn remove_parent_markers(value: &str) -> String {
    let mut cleaned = String::with_capacity(value.len());
    let mut previous_was_dot = false;
    for character in value.chars() {
        if character == '.' && previous_was_dot {
            cleaned.push('_');
            previous_was_dot = false;
            continue;
        }
        cleaned.push(character);
        previous_was_dot = character == '.';
    }
    cleaned
}

fn ensure_media_extension(value: &str) -> String {
    let extension = value.rsplit_once('.').map(|(_, extension)| extension);
    if extension.is_some_and(|extension| {
        SAFE_MEDIA_EXTENSIONS
            .iter()
            .any(|safe| extension.eq_ignore_ascii_case(safe))
    }) {
        return value.to_owned();
    }
    let stem_limit = MAX_FILENAME_CHARS - DEFAULT_EXTENSION.len();
    let stem = value.chars().take(stem_limit).collect::<String>();
    format!("{stem}{DEFAULT_EXTENSION}")
}

fn ascii_fallback(value: &str) -> String {
    let mapped = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, ' ' | '-' | '_' | '.') {
                return character;
            }
            '_'
        })
        .collect::<String>();
    let trimmed = mapped.trim_matches([' ', '.', '_']);
    if trimmed.is_empty() {
        return DEFAULT_FILENAME.to_owned();
    }
    trimmed.to_owned()
}

fn encode_extended_value(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if is_attribute_character(byte) {
            encoded.push(char::from(byte));
            continue;
        }
        encoded.push('%');
        encoded.push(char::from(
            HEX_DIGITS[usize::from(byte >> HIGH_NIBBLE_SHIFT)],
        ));
        encoded.push(char::from(HEX_DIGITS[usize::from(byte & LOW_NIBBLE_MASK)]));
    }
    encoded
}

fn is_attribute_character(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#' | b'$' | b'&' | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
        )
}
