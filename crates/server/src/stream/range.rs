use thiserror::Error;

pub const MAX_RANGE_HEADER_BYTES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteRange {
    Closed { start: u64, end: u64 },
    OpenEnded { start: u64 },
    Suffix { length: u64 },
}

impl std::fmt::Display for ByteRange {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closed { start, end } => write!(formatter, "bytes={start}-{end}"),
            Self::OpenEnded { start } => write!(formatter, "bytes={start}-"),
            Self::Suffix { length } => write!(formatter, "bytes=-{length}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RangeParseError {
    #[error("Range header is empty")]
    Empty,
    #[error("Range header exceeds the size limit")]
    TooLong,
    #[error("only the bytes range unit is supported")]
    UnsupportedUnit,
    #[error("multiple ranges are not supported")]
    MultipleRanges,
    #[error("Range header has invalid syntax")]
    InvalidSyntax,
    #[error("Range positions must contain ASCII digits only")]
    NonDigit,
    #[error("Range position exceeds u64")]
    Overflow,
    #[error("Range end precedes its start")]
    Reversed,
    #[error("suffix Range length must be greater than zero")]
    ZeroSuffix,
}

pub fn parse_single_range(value: &str) -> Result<ByteRange, RangeParseError> {
    if value.is_empty() {
        return Err(RangeParseError::Empty);
    }
    if value.len() > MAX_RANGE_HEADER_BYTES {
        return Err(RangeParseError::TooLong);
    }
    let (unit, range) = value
        .split_once('=')
        .ok_or(RangeParseError::InvalidSyntax)?;
    if unit != "bytes" {
        return Err(RangeParseError::UnsupportedUnit);
    }
    if range.is_empty() {
        return Err(RangeParseError::Empty);
    }
    if range.contains(',') {
        return Err(RangeParseError::MultipleRanges);
    }
    let (start, end) = range
        .split_once('-')
        .ok_or(RangeParseError::InvalidSyntax)?;
    if start.is_empty() && end.is_empty() {
        return Err(RangeParseError::Empty);
    }
    if start.is_empty() {
        let length = parse_position(end)?;
        if length == 0 {
            return Err(RangeParseError::ZeroSuffix);
        }
        return Ok(ByteRange::Suffix { length });
    }
    let start = parse_position(start)?;
    if end.is_empty() {
        return Ok(ByteRange::OpenEnded { start });
    }
    let end = parse_position(end)?;
    if end < start {
        return Err(RangeParseError::Reversed);
    }

    Ok(ByteRange::Closed { start, end })
}

fn parse_position(value: &str) -> Result<u64, RangeParseError> {
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(RangeParseError::NonDigit);
    }
    value.parse().map_err(|_| RangeParseError::Overflow)
}
