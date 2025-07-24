ALTER table pos_rewards ADD CONSTRAINT pos_rewards_owner_validator_id_key UNIQUE (owner, validator_id);
ALTER table pos_rewards DROP CONSTRAINT pos_rewards_owner_validator_id_epoch_key;
ALTER TABLE pos_rewards DROP COLUMN epoch;
ALTER TABLE pos_rewards DROP COLUMN claimed;