-- Create table for Points of Interest
CREATE TABLE poi (
    id SERIAL PRIMARY KEY
);

-- -- Create table for S2 cells with unique cell IDs
-- CREATE TABLE s2_cells (
--     cell_id BIGINT NOT NULL UNIQUE
-- );

-- Create junction table to link POIs to S2 cells
CREATE TABLE poi_s2_cells (
    poi_id INTEGER NOT NULL,
    cell_id BIGINT NOT NULL
);

-- -- Create table for keywords
-- CREATE TABLE keywords (
--     id SERIAL PRIMARY KEY,
--     word TEXT NOT NULL
-- );

-- Create junction table for POI and keywords
CREATE TABLE poi_keywords (
    poi_id INTEGER NOT NULL,
    keyword_id INTEGER NOT NULL
);

-- CREATE UNIQUE INDEX ON keywords (word);
