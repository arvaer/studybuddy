-- Store per-page text as a JSONB array for page-aware reading
ALTER TABLE resources ADD COLUMN content_pages JSONB;
