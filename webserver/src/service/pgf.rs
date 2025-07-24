use orm::pgf::{PaymentKindDb, PaymentRecurrenceDb};
use shared::id::Id;

use crate::appstate::AppState;
use crate::entity::pgf::{PaymentKind, PaymentRecurrence, PgfPayment};
use crate::error::pgf::PgfError;
use crate::repository::pgf::{PgfRepo, PgfRepoTrait};

#[derive(Clone)]
pub struct PgfService {
    pgf_repo: PgfRepo,
}

impl PgfService {
    pub fn new(app_state: AppState) -> Self {
        Self {
            pgf_repo: PgfRepo::new(app_state.clone()),
        }
    }

    pub async fn get_all_pgf_payments(
        &self,
        page: u64,
    ) -> Result<(Vec<PgfPayment>, u64, u64), PgfError> {
        let (payments, total_pages, total_items) = self
            .pgf_repo
            .get_pgf_continuous_payments(page as i64)
            .await
            .map_err(PgfError::Database)?;

        let payments = payments
            .into_iter()
            .map(|payment| PgfPayment {
                recurrence: match payment.payment_recurrence {
                    PaymentRecurrenceDb::Continuous => {
                        PaymentRecurrence::Continuous
                    }
                    PaymentRecurrenceDb::Retro => PaymentRecurrence::Retro,
                },
                proposal_id: payment.proposal_id as u64,
                kind: match payment.payment_kind {
                    PaymentKindDb::Ibc => PaymentKind::Ibc,
                    PaymentKindDb::Native => PaymentKind::Native,
                },
                receipient: Id::Account(payment.receipient),
                amount: payment.amount.into(),
            })
            .collect();

        Ok((payments, total_pages as u64, total_items as u64))
    }

    pub async fn find_pfg_payments_by_proposal_id(
        &self,
        proposal_id: u64,
    ) -> Result<Vec<PgfPayment>, PgfError> {
        let payment = self
            .pgf_repo
            .find_pgf_payments_by_proposal_id(proposal_id as i32)
            .await
            .map_err(PgfError::Database)?
            .into_iter()
            .map(|payment| PgfPayment {
                recurrence: match payment.payment_recurrence {
                    PaymentRecurrenceDb::Continuous => {
                        PaymentRecurrence::Continuous
                    }
                    PaymentRecurrenceDb::Retro => PaymentRecurrence::Retro,
                },
                proposal_id: payment.proposal_id as u64,
                kind: match payment.payment_kind {
                    PaymentKindDb::Ibc => PaymentKind::Ibc,
                    PaymentKindDb::Native => PaymentKind::Native,
                },
                receipient: Id::Account(payment.receipient),
                amount: payment.amount.into(),
            })
            .collect();

        Ok(payment)
    }
}
