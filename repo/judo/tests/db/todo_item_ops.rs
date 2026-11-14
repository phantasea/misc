use crate::helpers::db::setup_test_db;
use anyhow::Result;
use chrono::{Duration, Utc};
use judo::db::models::{NewTodoItem, NewTodoList, Priority, TodoItem, TodoList};

#[tokio::test]
async fn test_todo_item_crud_operations() -> Result<()> {
    // Set up test database
    let pool = setup_test_db().await?;

    // Create a new todo list
    let new_list = NewTodoList {
        name: "Test Shopping List".to_string(),
    };

    let created_list = TodoList::create(&pool, new_list).await?;

    // Create a new todo item with all fields
    let new_item_with_all_fields = NewTodoItem {
        list_id: created_list.id,
        name: "My item".to_string(),
        priority: Some(Priority::High),
        due_date: Some(Utc::now()),
    };

    // Create a new todo item without due date
    let new_item_without_due_date = NewTodoItem {
        list_id: created_list.id,
        name: "My item without date".to_string(),
        priority: Some(Priority::Low),
        due_date: None,
    };

    let mut created_item_with_all_fields =
        TodoItem::create(&pool, new_item_with_all_fields).await?;

    // Verify the created list has correct properties
    assert!(created_item_with_all_fields.id > 0);
    assert_eq!(created_item_with_all_fields.list_id, created_list.id);
    assert_eq!(created_item_with_all_fields.is_done, false);
    assert_eq!(created_item_with_all_fields.name, "My item");
    assert_eq!(created_item_with_all_fields.priority, Some(Priority::High));
    assert!(created_item_with_all_fields.due_date.is_some());
    assert!(created_item_with_all_fields.created_at <= Utc::now());
    assert_eq!(
        created_item_with_all_fields.created_at,
        created_item_with_all_fields.updated_at
    );

    let created_item_without_due_date = TodoItem::create(&pool, new_item_without_due_date).await?;

    // Verify the created list has correct properties
    assert!(created_item_without_due_date.id > 0);
    assert_eq!(created_item_without_due_date.list_id, created_list.id);
    assert_eq!(created_item_without_due_date.name, "My item without date");
    assert_eq!(created_item_without_due_date.is_done, false);
    assert_eq!(created_item_without_due_date.priority, Some(Priority::Low));
    assert!(created_item_without_due_date.due_date.is_none());
    assert!(created_item_without_due_date.created_at <= Utc::now());
    assert_eq!(
        created_item_without_due_date.created_at,
        created_item_without_due_date.updated_at
    );

    // Fetch items in the list
    let fetched_items_by_list = TodoItem::get_by_list_id(&pool, created_list.id).await?;
    assert_eq!(fetched_items_by_list.len(), 2);

    // Fetch specific item
    let fetched_item = TodoItem::get_by_id(&pool, created_item_with_all_fields.id)
        .await?
        .expect("Item should exist");
    assert_eq!(fetched_item.name, "My item");

    // Update name
    let new_name = "New name".to_string();
    created_item_with_all_fields
        .update_name(&pool, new_name.clone())
        .await?;
    let fetched_item_after_name_change =
        TodoItem::get_by_id(&pool, created_item_with_all_fields.id)
            .await?
            .expect("Item should exist");

    assert_eq!(created_item_with_all_fields.name, new_name);
    assert_eq!(fetched_item_after_name_change.name, new_name);

    // Toggle done
    let initial_state: bool = created_item_with_all_fields.is_done;
    created_item_with_all_fields.toggle_done(&pool).await?;
    let fetched_item_after_done_toggle =
        TodoItem::get_by_id(&pool, created_item_with_all_fields.id)
            .await?
            .expect("Item should exist");

    assert_eq!(
        fetched_item_after_done_toggle.is_done,
        created_item_with_all_fields.is_done
    );
    assert_eq!(fetched_item_after_done_toggle.is_done, !initial_state);

    // Update priority
    let original_priority = created_item_with_all_fields.priority.clone();
    let new_priority = Priority::Medium;
    created_item_with_all_fields
        .update_priority(&pool, new_priority.clone())
        .await?;
    let fetched_item_after_priority_change =
        TodoItem::get_by_id(&pool, created_item_with_all_fields.id)
            .await?
            .expect("Item should exist");

    assert_eq!(
        created_item_with_all_fields.priority,
        Some(new_priority.clone())
    );
    assert_eq!(
        fetched_item_after_priority_change.priority,
        Some(new_priority)
    );
    assert_ne!(created_item_with_all_fields.priority, original_priority);

    // Update due date
    let original_due_date = created_item_with_all_fields.due_date;
    let new_due_date = Utc::now() + Duration::days(7);
    created_item_with_all_fields
        .update_due_date(&pool, new_due_date)
        .await?;
    let fetched_item_after_due_date_change =
        TodoItem::get_by_id(&pool, created_item_with_all_fields.id)
            .await?
            .expect("Item should exist");

    assert_eq!(created_item_with_all_fields.due_date, Some(new_due_date));
    assert_eq!(
        fetched_item_after_due_date_change.due_date,
        Some(new_due_date)
    );
    assert_ne!(created_item_with_all_fields.due_date, original_due_date);

    // Delete the item
    let item_id_to_delete = created_item_with_all_fields.id;
    created_item_with_all_fields.delete(&pool).await?;

    // Verify item was deleted
    let deleted_item = TodoItem::get_by_id(&pool, item_id_to_delete).await?;
    assert!(deleted_item.is_none());

    // Verify only one item remains in the list
    let remaining_items = TodoItem::get_by_list_id(&pool, created_list.id).await?;
    assert_eq!(remaining_items.len(), 1);
    assert_eq!(remaining_items[0].name, "My item without date");

    Ok(())
}

#[tokio::test]
async fn test_todo_item_creation_edge_cases() -> Result<()> {
    let pool = setup_test_db().await?;

    // Create a test list
    let new_list = NewTodoList {
        name: "Test List".to_string(),
    };
    let created_list = TodoList::create(&pool, new_list).await?;

    // Test creating item with minimal fields (no priority, no due date)
    let minimal_item = NewTodoItem {
        list_id: created_list.id,
        name: "Minimal item".to_string(),
        priority: None, // Priority is now optional
        due_date: None,
    };
    let created_minimal = TodoItem::create(&pool, minimal_item).await?;
    assert_eq!(created_minimal.name, "Minimal item");
    assert_eq!(created_minimal.priority, None);
    assert_eq!(created_minimal.due_date, None);
    assert_eq!(created_minimal.is_done, false);

    // Test creating item with empty name
    let empty_name_item = NewTodoItem {
        list_id: created_list.id,
        name: "".to_string(),
        priority: Some(Priority::High),
        due_date: None,
    };
    let created_empty_name = TodoItem::create(&pool, empty_name_item).await?;
    assert_eq!(created_empty_name.name, "");

    // Test creating item with very long name
    let long_name = "A".repeat(1000);
    let long_name_item = NewTodoItem {
        list_id: created_list.id,
        name: long_name.clone(),
        priority: Some(Priority::Low),
        due_date: None,
    };
    let created_long_name = TodoItem::create(&pool, long_name_item).await?;
    assert_eq!(created_long_name.name, long_name);

    // Test creating item with past due date
    let past_date = Utc::now() - Duration::days(30);
    let past_due_item = NewTodoItem {
        list_id: created_list.id,
        name: "Past due item".to_string(),
        priority: Some(Priority::High),
        due_date: Some(past_date),
    };
    let created_past_due = TodoItem::create(&pool, past_due_item).await?;
    assert_eq!(created_past_due.due_date, Some(past_date));

    // Test creating item with far future due date
    let future_date = Utc::now() + Duration::days(365 * 10); // 10 years in the future
    let future_due_item = NewTodoItem {
        list_id: created_list.id,
        name: "Far future item".to_string(),
        priority: Some(Priority::Medium),
        due_date: Some(future_date),
    };
    let created_future_due = TodoItem::create(&pool, future_due_item).await?;
    assert_eq!(created_future_due.due_date, Some(future_date));

    Ok(())
}

#[tokio::test]
async fn test_todo_item_fetching_edge_cases() -> Result<()> {
    let pool = setup_test_db().await?;

    // Test fetching item with non-existent ID
    let non_existent_item = TodoItem::get_by_id(&pool, 99999).await?;
    assert!(non_existent_item.is_none());

    // Test fetching items from non-existent list
    let non_existent_list_items = TodoItem::get_by_list_id(&pool, 99999).await?;
    assert!(non_existent_list_items.is_empty());

    // Test fetching items from empty list
    let empty_list = NewTodoList {
        name: "Empty List".to_string(),
    };
    let created_empty_list = TodoList::create(&pool, empty_list).await?;
    let empty_list_items = TodoItem::get_by_list_id(&pool, created_empty_list.id).await?;
    assert!(empty_list_items.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_todo_item_priority_operations() -> Result<()> {
    let pool = setup_test_db().await?;

    let new_list = NewTodoList {
        name: "Priority Test List".to_string(),
    };
    let created_list = TodoList::create(&pool, new_list).await?;

    // Create item without priority
    let mut item_without_priority = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: created_list.id,
            name: "Item without priority".to_string(),
            priority: None, // Priority is now optional
            due_date: None,
        },
    )
    .await?;

    // Update from None to High priority
    item_without_priority
        .update_priority(&pool, Priority::High)
        .await?;
    assert_eq!(item_without_priority.priority, Some(Priority::High));

    // Update from High to Medium priority
    item_without_priority
        .update_priority(&pool, Priority::Medium)
        .await?;
    assert_eq!(item_without_priority.priority, Some(Priority::Medium));

    // Update from Medium to Low priority
    item_without_priority
        .update_priority(&pool, Priority::Low)
        .await?;
    assert_eq!(item_without_priority.priority, Some(Priority::Low));

    // Update back to High priority
    item_without_priority
        .update_priority(&pool, Priority::High)
        .await?;
    assert_eq!(item_without_priority.priority, Some(Priority::High));

    // Verify the priority change persists in database
    let fetched_item = TodoItem::get_by_id(&pool, item_without_priority.id)
        .await?
        .expect("Item should exist");
    assert_eq!(fetched_item.priority, Some(Priority::High));

    Ok(())
}

#[tokio::test]
async fn test_todo_item_due_date_operations() -> Result<()> {
    let pool = setup_test_db().await?;

    let new_list = NewTodoList {
        name: "Due Date Test List".to_string(),
    };
    let created_list = TodoList::create(&pool, new_list).await?;

    // Create item without due date
    let mut item_without_due_date = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: created_list.id,
            name: "Item without due date".to_string(),
            priority: Some(Priority::Medium),
            due_date: None,
        },
    )
    .await?;

    // Add due date to item that didn't have one
    let first_due_date = Utc::now() + Duration::days(1);
    item_without_due_date
        .update_due_date(&pool, first_due_date)
        .await?;
    assert_eq!(item_without_due_date.due_date, Some(first_due_date));

    // Update due date to a different date
    let second_due_date = Utc::now() + Duration::days(7);
    item_without_due_date
        .update_due_date(&pool, second_due_date)
        .await?;
    assert_eq!(item_without_due_date.due_date, Some(second_due_date));

    // Update to past due date
    let past_due_date = Utc::now() - Duration::days(1);
    item_without_due_date
        .update_due_date(&pool, past_due_date)
        .await?;
    assert_eq!(item_without_due_date.due_date, Some(past_due_date));

    // Update to far future date
    let far_future_date = Utc::now() + Duration::days(365);
    item_without_due_date
        .update_due_date(&pool, far_future_date)
        .await?;
    assert_eq!(item_without_due_date.due_date, Some(far_future_date));

    // Verify the due date change persists in database
    let fetched_item = TodoItem::get_by_id(&pool, item_without_due_date.id)
        .await?
        .expect("Item should exist");
    assert_eq!(fetched_item.due_date, Some(far_future_date));

    Ok(())
}

#[tokio::test]
async fn test_todo_item_toggle_done_edge_cases() -> Result<()> {
    let pool = setup_test_db().await?;

    let new_list = NewTodoList {
        name: "Toggle Done Test List".to_string(),
    };
    let created_list = TodoList::create(&pool, new_list).await?;

    let mut test_item = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: created_list.id,
            name: "Toggle test item".to_string(),
            priority: Some(Priority::Medium),
            due_date: None,
        },
    )
    .await?;

    // Initial state should be false
    assert_eq!(test_item.is_done, false);

    // Toggle multiple times to test both directions
    for i in 0..10 {
        let expected_state = (i + 1) % 2 == 1; // After first toggle (i=0), should be true
        test_item.toggle_done(&pool).await?;
        assert_eq!(test_item.is_done, expected_state);

        // Verify state persists in database
        let fetched_item = TodoItem::get_by_id(&pool, test_item.id)
            .await?
            .expect("Item should exist");
        assert_eq!(fetched_item.is_done, expected_state);
    }

    Ok(())
}

#[tokio::test]
async fn test_todo_item_name_update_edge_cases() -> Result<()> {
    let pool = setup_test_db().await?;

    let new_list = NewTodoList {
        name: "Name Update Test List".to_string(),
    };
    let created_list = TodoList::create(&pool, new_list).await?;

    let mut test_item = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: created_list.id,
            name: "Original name".to_string(),
            priority: Some(Priority::Medium),
            due_date: None,
        },
    )
    .await?;

    // Update to empty string
    test_item.update_name(&pool, "".to_string()).await?;
    assert_eq!(test_item.name, "");

    // Update to very long name
    let long_name = "Very ".repeat(200) + "long name";
    test_item.update_name(&pool, long_name.clone()).await?;
    assert_eq!(test_item.name, long_name);

    // Update to name with special characters
    let special_name = "Special chars: !@#$%^&*()_+-=[]{}|;':\",./<>?`~".to_string();
    test_item.update_name(&pool, special_name.clone()).await?;
    assert_eq!(test_item.name, special_name);

    // Update to name with unicode characters
    let unicode_name = "Unicode: 🚀 测试 🎉 café naïve résumé".to_string();
    test_item.update_name(&pool, unicode_name.clone()).await?;
    assert_eq!(test_item.name, unicode_name);

    // Update to name with newlines and tabs
    let multiline_name = "Line 1\nLine 2\tTabbed".to_string();
    test_item.update_name(&pool, multiline_name.clone()).await?;
    assert_eq!(test_item.name, multiline_name);

    // Verify final state persists in database
    let fetched_item = TodoItem::get_by_id(&pool, test_item.id)
        .await?
        .expect("Item should exist");
    assert_eq!(fetched_item.name, multiline_name);

    Ok(())
}

#[tokio::test]
async fn test_todo_item_deletion_scenarios() -> Result<()> {
    let pool = setup_test_db().await?;

    let new_list = NewTodoList {
        name: "Deletion Test List".to_string(),
    };
    let created_list = TodoList::create(&pool, new_list).await?;

    // Create multiple items for deletion testing
    let item1 = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: created_list.id,
            name: "Item 1".to_string(),
            priority: Some(Priority::High),
            due_date: Some(Utc::now() + Duration::days(1)),
        },
    )
    .await?;

    let item2 = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: created_list.id,
            name: "Item 2".to_string(),
            priority: Some(Priority::Medium),
            due_date: None,
        },
    )
    .await?;

    let item3 = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: created_list.id,
            name: "Item 3".to_string(),
            priority: None, // No priority for this item
            due_date: Some(Utc::now() - Duration::days(1)),
        },
    )
    .await?;

    // Verify all items exist
    let all_items = TodoItem::get_by_list_id(&pool, created_list.id).await?;
    assert_eq!(all_items.len(), 3);

    // Delete middle item
    let item2_id = item2.id;
    item2.delete(&pool).await?;

    // Verify item2 is deleted and others remain
    let remaining_items = TodoItem::get_by_list_id(&pool, created_list.id).await?;
    assert_eq!(remaining_items.len(), 2);

    let deleted_item = TodoItem::get_by_id(&pool, item2_id).await?;
    assert!(deleted_item.is_none());

    // Verify specific items still exist
    let item1_exists = TodoItem::get_by_id(&pool, item1.id).await?;
    assert!(item1_exists.is_some());

    let item3_exists = TodoItem::get_by_id(&pool, item3.id).await?;
    assert!(item3_exists.is_some());

    // Delete remaining items
    item1.delete(&pool).await?;
    item3.delete(&pool).await?;

    // Verify list is now empty
    let final_items = TodoItem::get_by_list_id(&pool, created_list.id).await?;
    assert_eq!(final_items.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_todo_item_timestamp_updates() -> Result<()> {
    let pool = setup_test_db().await?;

    let new_list = NewTodoList {
        name: "Timestamp Test List".to_string(),
    };
    let created_list = TodoList::create(&pool, new_list).await?;

    let mut test_item = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: created_list.id,
            name: "Timestamp test".to_string(),
            priority: Some(Priority::Medium),
            due_date: None,
        },
    )
    .await?;

    let original_created_at = test_item.created_at;
    let original_updated_at = test_item.updated_at;

    // Small delay to ensure timestamp difference
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Update name and verify timestamp changes
    test_item
        .update_name(&pool, "Updated name".to_string())
        .await?;
    assert_eq!(test_item.created_at, original_created_at); // Created should not change
    assert!(test_item.updated_at > original_updated_at); // Updated should change

    let after_name_update = test_item.updated_at;
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Update priority and verify timestamp changes
    test_item.update_priority(&pool, Priority::High).await?;
    assert_eq!(test_item.created_at, original_created_at); // Created should not change
    assert!(test_item.updated_at > after_name_update); // Updated should change again

    let after_priority_update = test_item.updated_at;
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Toggle done and verify timestamp changes
    test_item.toggle_done(&pool).await?;
    assert_eq!(test_item.created_at, original_created_at); // Created should not change
    assert!(test_item.updated_at > after_priority_update); // Updated should change again

    let after_toggle_update = test_item.updated_at;
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Update due date and verify timestamp changes
    test_item
        .update_due_date(&pool, Utc::now() + Duration::days(1))
        .await?;
    assert_eq!(test_item.created_at, original_created_at); // Created should not change
    assert!(test_item.updated_at > after_toggle_update); // Updated should change again

    Ok(())
}

#[tokio::test]
async fn test_todo_item_invalid_list_id() -> Result<()> {
    let pool = setup_test_db().await?;

    // Try to create item with non-existent list_id
    let invalid_item = NewTodoItem {
        list_id: 99999, // Non-existent list
        name: "Item for non-existent list".to_string(),
        priority: Some(Priority::Medium),
        due_date: None,
    };

    // This should fail due to foreign key constraint
    let result = TodoItem::create(&pool, invalid_item).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_todo_item_ordering_by_creation_date() -> Result<()> {
    let pool = setup_test_db().await?;

    let new_list = NewTodoList {
        name: "Ordering Test List".to_string(),
    };
    let created_list = TodoList::create(&pool, new_list).await?;

    // Create items with small delays to ensure different creation times
    let item1 = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: created_list.id,
            name: "First Item".to_string(),
            priority: Some(Priority::High),
            due_date: None,
        },
    )
    .await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let item2 = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: created_list.id,
            name: "Second Item".to_string(),
            priority: Some(Priority::Medium),
            due_date: None,
        },
    )
    .await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let item3 = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: created_list.id,
            name: "Third Item".to_string(),
            priority: Some(Priority::Low),
            due_date: None,
        },
    )
    .await?;

    // Get items - should be ordered by created_at
    let all_items = TodoItem::get_by_list_id(&pool, created_list.id).await?;
    assert_eq!(all_items.len(), 3);

    // Verify ordering
    assert_eq!(all_items[0].name, "First Item");
    assert_eq!(all_items[1].name, "Second Item");
    assert_eq!(all_items[2].name, "Third Item");

    // Verify timestamps are in order
    assert!(all_items[0].created_at <= all_items[1].created_at);
    assert!(all_items[1].created_at <= all_items[2].created_at);

    // Verify IDs match
    assert_eq!(all_items[0].id, item1.id);
    assert_eq!(all_items[1].id, item2.id);
    assert_eq!(all_items[2].id, item3.id);

    Ok(())
}
