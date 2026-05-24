-- Optional ChromaDB index metadata.
--
-- Populated only when the user enables [chromadb] in chatur.toml and indexes
-- a project. The actual vectors live inside the chroma server's own data
-- directory; this table just tracks which collections we have created and
-- when each was last refreshed.

CREATE TABLE IF NOT EXISTS chroma_indexes (
    project_id       TEXT PRIMARY KEY REFERENCES projects (id) ON DELETE CASCADE,
    collection_name  TEXT NOT NULL,
    last_indexed_at  TEXT,
    file_count       INTEGER NOT NULL DEFAULT 0,
    chunk_count      INTEGER NOT NULL DEFAULT 0,
    data             TEXT NOT NULL DEFAULT '{}'
);
