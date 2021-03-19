use std::convert::TryInto;

tonic::include_proto!("paas_types");

impl From<uuid::Uuid> for Uuid {
    fn from(foreign: uuid::Uuid) -> Self {
        Self {
            id: foreign.as_bytes().to_vec(),
        }
    }
}
impl TryInto<uuid::Uuid> for Uuid {
    type Error = uuid::Error;

    fn try_into(self) -> Result<uuid::Uuid, Self::Error> {
        uuid::Uuid::from_slice(&self.id)
    }
}
