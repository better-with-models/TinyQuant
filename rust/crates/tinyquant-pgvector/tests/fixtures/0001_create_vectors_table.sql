CREATE EXTENSION IF NOT EXISTS vector;
CREATE TABLE IF NOT EXISTS vectors (
    id        TEXT PRIMARY KEY,
    embedding vector(768)
);
