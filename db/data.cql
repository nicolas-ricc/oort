
CREATE KEYSPACE IF NOT EXISTS store WITH REPLICATION =
{ 'class' : 'SimpleStrategy',
'replication_factor' : '1'
};

-- User concepts table
CREATE TABLE IF NOT EXISTS store.user_concepts (
    user_id UUID,
    concept_id UUID,
    concept_text TEXT,
    embedding_vector LIST<DOUBLE>, 
    created_at TIMESTAMP,
    PRIMARY KEY (user_id, concept_id)
);

-- Tracking concept sources
CREATE TABLE IF NOT EXISTS store.concept_sources (
    concept_id UUID,
    user_id UUID,
    source_text TEXT,
    source_type TEXT,
    created_at TIMESTAMP,
    PRIMARY KEY (concept_id, user_id)
);

-- Text References table to store uploaded texts and their associated concepts
CREATE TABLE IF NOT EXISTS store.text_references (
    text_id UUID PRIMARY KEY,
    user_id UUID,
    filename TEXT,
    url TEXT,
    concepts LIST<TEXT>,  -- List of concept strings contained in this text
    upload_timestamp TIMESTAMP,
    file_size INT
);

-- Index to efficiently query by concept
CREATE INDEX IF NOT EXISTS text_references_concepts_idx ON store.text_references (concepts);

-- Index to query by user
CREATE INDEX IF NOT EXISTS text_references_user_idx ON store.text_references (user_id);

-- Concept-to-text mapping table for efficient reverse lookups
CREATE TABLE IF NOT EXISTS store.concept_text_mapping (
    concept_text TEXT,
    user_id UUID,
    text_id UUID,
    filename TEXT,
    url TEXT,
    upload_timestamp TIMESTAMP,
    PRIMARY KEY ((concept_text, user_id), text_id)
) WITH CLUSTERING ORDER BY (text_id DESC);