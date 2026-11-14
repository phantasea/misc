-- Make priority field optional (nullable)
-- SQLite doesn't support ALTER COLUMN directly, so we need to recreate the table

-- Create a new temporary table with the updated schema
CREATE TABLE todo_items_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    list_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    is_done BOOLEAN NOT NULL DEFAULT FALSE,
    priority TEXT CHECK (priority IN ('high', 'medium', 'low') OR priority IS NULL),
    due_date TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (list_id) REFERENCES todo_lists (id) ON DELETE CASCADE
);

-- Copy data from old table to new table
INSERT INTO todo_items_new (id, list_id, name, is_done, priority, due_date, created_at, updated_at)
SELECT id, list_id, name, is_done, priority, due_date, created_at, updated_at
FROM todo_items;

-- Drop the old table
DROP TABLE todo_items;

-- Rename the new table to the original name
ALTER TABLE todo_items_new RENAME TO todo_items;

-- Recreate indexes
CREATE INDEX idx_todo_items_list_id ON todo_items(list_id);
CREATE INDEX idx_todo_items_priority ON todo_items(priority);
CREATE INDEX idx_todo_items_is_done ON todo_items(is_done); 