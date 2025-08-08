-- Create table for Points of Interest
CREATE TABLE poi (
    id SERIAL PRIMARY KEY,
    s2cell BIGINT NOT NULL,
    -- Only supports quadkeys up to 32 bits, which is fine at zoom 12.
    quadkey INTEGER NOT NULL,
    keywords INTEGER[]
);

CREATE INDEX poi_quadkey ON poi (quadkey);
CREATE INDEX poi_s2cell ON poi (s2cell);

-- Vocabulary for "words" column in poi table.
CREATE TABLE vocabulary (
    id INTEGER PRIMARY KEY NOT NULL,
    word TEXT NOT NULL
);

