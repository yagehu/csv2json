BEGIN;

CREATE TABLE document
  ( id      UUID PRIMARY KEY
  , content JSONB NOT NULL
  );

COMMIT;
