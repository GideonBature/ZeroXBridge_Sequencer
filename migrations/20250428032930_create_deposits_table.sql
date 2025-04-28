-- Create deposits table
CREATE TABLE IF NOT EXISTS deposits (
    id SERIAL PRIMARY KEY,
    user_address TEXT NOT NULL,
    amount BIGINT NOT NULL,
    l2_tx_id INTEGER,
    commitment_hash TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    retry_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Create deposit_proofs table
CREATE TABLE IF NOT EXISTS deposit_proofs (
    id SERIAL PRIMARY KEY,
    deposit_id INTEGER REFERENCES deposits(id),
    proof_params BYTEA,
    proof_data BYTEA,
    status TEXT NOT NULL DEFAULT 'ready',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);
