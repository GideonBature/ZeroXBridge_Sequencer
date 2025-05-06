-- Add migration script here
-- Create withdrawals table

CREATE TABLE IF NOT EXISTS withdrawals (
    id SERIAL PRIMARY KEY,
    stark_pub_key TEXT NOT NULL,
    amount BIGINT NOT NULL,
    l1_token TEXT NOT NULL,
    commitment_hash TEXT NOT NULL,
    status TEXT NOT NULL,
    retry_count INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create deposits table
CREATE TABLE IF NOT EXISTS deposits (
    id SERIAL PRIMARY KEY,
    stark_pub_key TEXT NOT NULL,
    amount BIGINT NOT NULL,
    commitment_hash TEXT NOT NULL,
    status TEXT NOT NULL,
    retry_count INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create L2 transactions table
CREATE TABLE IF NOT EXISTS l2_transactions (
    id BIGSERIAL PRIMARY KEY,
    stark_pub_key VARCHAR(42) NOT NULL,
    amount BIGINT NOT NULL,
    token_address VARCHAR(42) NOT NULL,
    status VARCHAR NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    retry_count INT NOT NULL DEFAULT 0,
    tx_hash VARCHAR(66),
    error TEXT,
    proof_data TEXT
);

-- Create index on status for faster queries
CREATE INDEX IF NOT EXISTS l2_transactions_status_idx ON l2_transactions (status);

-- Create index on created_at for sorting
CREATE INDEX IF NOT EXISTS l2_transactions_created_at_idx ON l2_transactions (created_at);