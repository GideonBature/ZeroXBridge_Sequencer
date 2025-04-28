-- Add migration script here
-- Create withdrawals table

DROP TABLE IF EXISTS withdrawals;
CREATE TABLE withdrawals (
    id SERIAL PRIMARY KEY,
    stark_pub_key TEXT NOT NULL,
    amount BIGINT NOT NULL,
    l1_token TEXT NOT NULL,
    commitment_hash TEXT NOT NULL,
    status TEXT NOT NULL,
    retry_count INT NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT now()
);

-- Create deposits table
DROP TABLE IF EXISTS deposits;
CREATE TABLE deposits (
    id SERIAL PRIMARY KEY,
    user_address TEXT NOT NULL,
    amount BIGINT NOT NULL,
    commitment_hash TEXT NOT NULL,
    status TEXT NOT NULL,
    retry_count INT NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT now()
);
