
CREATE KEYSPACE IF NOT EXISTS store WITH REPLICATION =
{ 'class' : 'SimpleStrategy',
'replication_factor' : '1'
};

-- User concepts table
CREATE TABLE IF NOT EXISTS user_concepts (
    user_id UUID,
    concept_id UUID,
    concept_text TEXT,
    embedding_vector LIST<FLOAT>,
    created_at TIMESTAMP,
    PRIMARY KEY (user_id, concept_id)
);

-- Tracking concept sources
CREATE TABLE IF NOT EXISTS concept_sources (
    concept_id UUID,
    user_id UUID,
    source_text TEXT,
    source_type TEXT,
    created_at TIMESTAMP,
    PRIMARY KEY (concept_id, user_id)
);