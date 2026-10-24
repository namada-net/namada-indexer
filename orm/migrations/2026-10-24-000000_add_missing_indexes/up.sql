CREATE INDEX IF NOT EXISTS index_ibc_ack_tx_hash ON ibc_ack (tx_hash);
CREATE INDEX IF NOT EXISTS index_inner_transactions_wrapper_id ON inner_transactions (wrapper_id);
