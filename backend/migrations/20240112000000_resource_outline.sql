-- A source's table of contents (#58), from the PDF's bookmarks: a JSON list
-- of {title, page, children}. Filled the first time the coach walks it and
-- kept, since a resource's file never changes; NULL until then, [] for a
-- file that has none.
ALTER TABLE resources ADD COLUMN outline JSONB;
