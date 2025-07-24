-- This file should undo anything in `up.sql`

-- Step 1: Rename the existing enum type
ALTER TYPE CRAWLER_NAME RENAME TO CRAWLER_NAME_OLD;

-- Step 2: Create the new enum type without the added values
CREATE TYPE CRAWLER_NAME AS ENUM ('chain', 'governance', 'parameters', 'pos', 'rewards', 'transactions');

-- Step 3: Update all columns to use the new enum type
ALTER TABLE crawler_state ALTER COLUMN name TYPE CRAWLER_NAME
USING kind::text::CRAWLER_NAME;

-- Step 4: Drop the old enum type
DROP TYPE CRAWLER_NAME_OLD;