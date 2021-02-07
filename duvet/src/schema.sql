BEGIN;

CREATE TABLE IF NOT EXISTS sources (
    id INTEGER PRIMARY KEY,
    hash BLOB UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS source_locations (
    id INTEGER PRIMARY KEY,
    location VARCHAR UNIQUE,
    source_id INTEGER NOT NULL,
    title VARCHAR
);

CREATE TABLE IF NOT EXISTS source_lines (
    source_id INTEGER,
    line INTEGER,
    columns INTEGER,
    offset INTEGER,
    PRIMARY KEY(source_id, line)
);

CREATE TABLE IF NOT EXISTS instances (
    id INTEGER PRIMARY KEY,
    name VARCHAR NOT NULL,
    source_location_id INTEGER NOT NULL,
    start_offset INTEGER NOT NULL,
    end_offset INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS annotations (
    id INTEGER PRIMARY KEY,
    type_id INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS annotation_labels (
    annotation_id INTEGER,
    label VARCHAR,
    PRIMARY KEY(annotation_id, label)
);

CREATE TABLE IF NOT EXISTS annotation_source_regions (
    annotation_id INTEGER NOT NULL,
    start_offset INTEGER NOT NULL,
    end_offset INTEGER NOT NULL,
    source_location_id INTEGER NOT NULL,
    PRIMARY KEY(annotation_id, start_offset, end_offset, source_location_id)
);

CREATE TABLE IF NOT EXISTS annotation_instance_regions (
    annotation_id INTEGER NOT NULL,
    start_offset INTEGER NOT NULL,
    end_offset INTEGER NOT NULL,
    instance_id INTEGER NOT NULL,
    PRIMARY KEY(annotation_id, start_offset, end_offset, instance_id)
);

CREATE TABLE IF NOT EXISTS annotation_relations (
    source_id INTEGER NOT NULL,
    target_id INTEGER NOT NULL,
    type_id INTEGER NOT NULL,
    PRIMARY KEY(source_id, target_id, type_id)
);

CREATE TABLE IF NOT EXISTS annotation_metrics (
    annotation_id INTEGER NOT NULL,
    name VARCHAR NOT NULL,
    value INTEGER NOT NULL,
    PRIMARY KEY(annotation_id, name)
);

END;
