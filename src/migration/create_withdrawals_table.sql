CREATE TYPE withdrawal_status AS ENUM ('pending', 'processed');
ALTER TABLE withdrawals
ALTER COLUMN status TYPE withdrawal_status
USING status::withdrawal_status;
ADD CONSTRAINT unique_commitment_hash UNIQUE (commitment_hash);

CREATE TABLE IF NOT EXISTS withdrawals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    stark_pub_key TEXT NOT NULL,
    amount TEXT NOT NULL,
    commitment_hash TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ,
    CONSTRAINT status_check CHECK (status IN ('pending', 'processed', 'failed'))
);

CREATE INDEX idx_withdrawals_status ON withdrawals(status);
CREATE INDEX idx_withdrawals_stark_pub_key ON withdrawals(stark_pub_key);

