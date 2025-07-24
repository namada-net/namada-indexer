use crate::appstate::AppState;
use crate::entity::masp::MaspPoolAggregate;
use crate::error::masp::MaspError;
use crate::repository::masp::{MaspRepository, MaspRepositoryTrait};

#[derive(Clone)]
pub struct MaspService {
    pub masp_repo: MaspRepository,
}

impl MaspService {
    pub fn new(app_state: AppState) -> Self {
        Self {
            masp_repo: MaspRepository::new(app_state),
        }
    }

    pub async fn find_all_masp_aggregates(
        &self,
        token: Option<String>,
    ) -> Result<Vec<MaspPoolAggregate>, MaspError> {
        let masp_aggregates = match token {
            Some(token) => self
                .masp_repo
                .find_all_aggregates_by_token(token)
                .await
                .map_err(MaspError::Database)?
                .into_iter()
                .map(MaspPoolAggregate::from)
                .collect(),
            None => self
                .masp_repo
                .find_all_aggregates()
                .await
                .map_err(MaspError::Database)?
                .into_iter()
                .map(MaspPoolAggregate::from)
                .collect(),
        };

        Ok(masp_aggregates)
    }
}
