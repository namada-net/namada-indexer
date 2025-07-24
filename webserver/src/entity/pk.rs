use orm::revealed_pk::RevealedPkDb;

#[derive(Clone, Debug)]
pub struct RevealedPk {
    pub public_key: Option<String>,
}

impl From<RevealedPkDb> for RevealedPk {
    fn from(value: RevealedPkDb) -> Self {
        Self {
            public_key: Some(value.pk),
        }
    }
}
