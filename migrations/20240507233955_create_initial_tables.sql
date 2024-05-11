-- Add migration script here
CREATE TABLE sample_entity
(
    id SERIAL PRIMARY KEY,
    text VARCHAR(512),
    created_at TIMESTAMP NOT NULL DEFAULT now()
);
