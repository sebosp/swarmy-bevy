//! Error handling of BevySC2MapError

use std::num::ParseFloatError;
use std::num::TryFromIntError;

use nom::error::ErrorKind;
use nom::error::ParseError;

/// Holds the result of parsing progress and the possibly failures
pub type BevySC2MapResult<I, O> = Result<(I, O), BevySC2MapError>;

#[derive(thiserror::Error, Debug)]
pub enum BevySC2MapError {
    /// Unable to parse the MPQ file, could be corrupted or not a replay file
    #[error("MPQ")]
    MPQ(#[from] nom_mpq::MPQParserError),
    /// Unable to parse the byte aligned data types
    #[error("Nom ByteAligned {0}")]
    ByteAligned(String),
    /// Unable to parse the MPQ file, could be corrupted or not a replay file
    #[error("S2Protocol")]
    S2Proto(#[from] s2protocol::S2ProtocolError),
    /// Unable to parse a value that should have been an integer
    #[error("TryFromInt")]
    ValueError(#[from] TryFromIntError),
    /// An I/O Error
    #[error("IO")]
    IoError(#[from] std::io::Error),
    /// Conversion to UTF-8 failed, from the `Vec<u8>` "name" fields in the proto fields
    #[error("Utf8 conversion")]
    Utf8Error(#[from] std::str::Utf8Error),
    /// Parse Float Error
    #[error("Parse float")]
    ParseFloat(#[from] ParseFloatError),
    /// Other error
    #[error("Other {0}")]
    Other(String),
}

/// Conversion of errors from byte aligned parser
impl<I> From<nom::Err<nom::error::Error<I>>> for BevySC2MapError
where
    I: Clone + std::fmt::Debug,
{
    fn from(err: nom::Err<nom::error::Error<I>>) -> Self {
        match err {
            nom::Err::Incomplete(_) => {
                unreachable!("This library is compatible with only complete parsers, not streaming")
            }
            nom::Err::Error(e) => BevySC2MapError::ByteAligned(format!("{e:?}")),
            nom::Err::Failure(e) => BevySC2MapError::ByteAligned(format!("{e:?}")),
        }
    }
}

impl<I> ParseError<I> for BevySC2MapError
where
    I: Clone,
{
    fn from_error_kind(_input: I, kind: ErrorKind) -> Self {
        BevySC2MapError::ByteAligned(format!("{kind:?}"))
    }

    fn append(_input: I, _kind: ErrorKind, other: Self) -> Self {
        other
    }
}
