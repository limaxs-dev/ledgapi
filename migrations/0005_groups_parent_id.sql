-- 0005_groups_parent_id.sql — add nesting to groups.
ALTER TABLE groups ADD COLUMN parent_id TEXT REFERENCES groups(id) ON DELETE CASCADE;
CREATE INDEX groups_parent_idx ON groups(parent_id);