UPDATE pos_rewards SET claimed = false WHERE claimed IS NULL;

ALTER TABLE pos_rewards ALTER COLUMN claimed SET NOT NULL;
