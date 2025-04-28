-- Create a table to track the last processed block for event watchers
CREATE TABLE IF NOT EXISTS block_trackers (
    key TEXT PRIMARY KEY,
    last_block BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add a comment to explain the purpose of this table
COMMENT ON TABLE block_trackers IS 'Tracks the last processed block numbers for different event watchers';
COMMENT ON COLUMN block_trackers.key IS 'Unique identifier for the event watcher';
COMMENT ON COLUMN block_trackers.last_block IS 'Last block number that was successfully processed';

-- Create an index on the key for quick lookups
CREATE INDEX idx_block_trackers_key ON block_trackers(key); 