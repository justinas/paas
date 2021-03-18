use std::convert::TryFrom;
use tonic::transport::Certificate;
use x509_parser::parse_x509_certificate;

// TODO: consider making field private,
// so it is only possible to construct a UserId from a cert.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserId(pub String);

impl TryFrom<&Certificate> for UserId {
    type Error = ();
    fn try_from(cert: &Certificate) -> Result<Self, Self::Error> {
        // TODO: proper error types
        let (_, cert) = parse_x509_certificate(cert.get_ref()).map_err(|_| ())?;
        let common_name = cert
            .subject()
            .iter_common_name()
            .next()
            .ok_or(())?
            .as_str()
            .map_err(|_| ())?;
        Ok(UserId(common_name.into()))
    }
}
