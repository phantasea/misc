-- Add ordering fields to both todo_lists and todo_items tables
-- The ordering will be based on the current creation order (preserving existing order)

-- Add ordering field to todo_lists
ALTER TABLE todo_lists ADD COLUMN ordering INTEGER NOT NULL DEFAULT 0;

-- Set initial ordering values based on creation date (preserving current order)
UPDATE todo_lists SET ordering = (
    SELECT COUNT(*) FROM todo_lists t2 WHERE t2.created_at <= todo_lists.created_at
);

-- Add ordering field to todo_items  
ALTER TABLE todo_items ADD COLUMN ordering INTEGER NOT NULL DEFAULT 0;

-- Set initial ordering values based on creation date within each list
UPDATE todo_items SET ordering = (
    SELECT COUNT(*) FROM todo_items t2 
    WHERE t2.list_id = todo_items.list_id 
    AND t2.created_at <= todo_items.created_at
);

-- Create indexes for better performance on ordering queries
CREATE INDEX idx_todo_lists_ordering ON todo_lists(ordering);
CREATE INDEX idx_todo_items_list_ordering ON todo_items(list_id, ordering);
