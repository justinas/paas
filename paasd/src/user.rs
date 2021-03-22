use std::convert::TryFrom;
use tonic::{transport::Certificate, Status};
use x509_parser::{error::X509Error, nom::Finish, parse_x509_certificate};

/// The error produced by unsuccessful authentication of a user from a client certificate.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("malformed X509 certificate")]
    X509(#[from] X509Error),
    #[error("no common name found in the certificate")]
    NoCommonName,
}

impl Into<Status> for AuthError {
    fn into(self) -> Status {
        Status::unauthenticated(format!("{}", self))
    }
}

/// Represents a user that has been successfully authenticated via a TLS client certificate.
/// Wraps the user's common name, extracted from the certificate.
// TODO: consider making field private,
// so it is only possible to construct a UserId from a cert.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserId(pub String);

impl TryFrom<&Certificate> for UserId {
    type Error = AuthError;

    fn try_from(cert: &Certificate) -> Result<Self, Self::Error> {
        let (_, cert) = parse_x509_certificate(cert.get_ref()).finish()?;
        let common_name = cert
            .subject()
            .iter_common_name()
            .next()
            .ok_or(AuthError::NoCommonName)?
            .as_str()?;
        Ok(UserId(common_name.into()))
    }
}
