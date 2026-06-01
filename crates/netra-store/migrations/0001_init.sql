-- NETRA initial schema.
--
-- Each entity is persisted as a JSON `data` blob (the source of truth) plus a
-- few promoted columns used purely for indexing and foreign keys. Adding a
-- field to a domain struct therefore needs no migration — only changes that
-- affect querying or relations do.

CREATE TABLE projects (
    id   TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    data TEXT NOT NULL
);

CREATE TABLE batches (
    id   TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    data TEXT NOT NULL
);

CREATE TABLE jobs (
    id         TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    batch_id   TEXT REFERENCES batches (id) ON DELETE SET NULL,
    status     TEXT NOT NULL,
    created_at TEXT NOT NULL,
    data       TEXT NOT NULL
);

CREATE INDEX idx_jobs_status ON jobs (status);
CREATE INDEX idx_jobs_project ON jobs (project_id);

CREATE TABLE batch_items (
    id       TEXT PRIMARY KEY,
    batch_id TEXT NOT NULL REFERENCES batches (id) ON DELETE CASCADE,
    data     TEXT NOT NULL
);

CREATE INDEX idx_batch_items_batch ON batch_items (batch_id);

CREATE TABLE templates (
    id   TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    data TEXT NOT NULL
);
