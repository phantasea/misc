use crate::helpers::db::setup_test_db;
use anyhow::Result;
use chrono::{Duration, Utc};
use judo::db::models::{NewTodoItem, NewTodoList, Priority, TodoItem, TodoList, UIList};

#[tokio::test]
async fn test_todo_list_crud_operations() -> Result<()> {
    // Set up test database
    let pool = setup_test_db().await?;

    // Create a new todo list
    let new_list = NewTodoList {
        name: "Test Shopping List".to_string(),
    };

    let created_list = TodoList::create(&pool, new_list).await?;

    // Verify the created list has correct properties
    assert_eq!(created_list.name, "Test Shopping List");
    assert!(created_list.id > 0);
    assert!(created_list.created_at <= Utc::now());
    assert_eq!(created_list.created_at, created_list.updated_at);

    // Test 2: Get the list by ID
    let fetched_list = TodoList::get_by_id(&pool, created_list.id)
        .await?
        .expect("List should exist");

    assert_eq!(fetched_list.id, created_list.id);
    assert_eq!(fetched_list.name, created_list.name);
    assert_eq!(fetched_list.created_at, created_list.created_at);

    // Test 3: Update the list name
    let mut mutable_list = fetched_list;
    let new_name = "Updated Shopping List".to_string();
    mutable_list.update_name(&pool, new_name.clone()).await?;

    assert_eq!(mutable_list.name, new_name);
    assert!(mutable_list.updated_at > mutable_list.created_at);

    // Test 4: Verify the update persisted in database
    let updated_from_db = TodoList::get_by_id(&pool, mutable_list.id)
        .await?
        .expect("List should still exist");

    assert_eq!(updated_from_db.name, new_name);
    assert_eq!(updated_from_db.updated_at, mutable_list.updated_at);

    // Test 5: Get all lists (should contain our list)
    let all_lists = TodoList::get_all(&pool).await?;
    assert_eq!(all_lists.len(), 1);
    assert_eq!(all_lists[0].name, new_name);

    // Test 6: Delete the list
    updated_from_db.delete(&pool).await?;

    // Test 7: Verify deletion
    let deleted_list = TodoList::get_by_id(&pool, mutable_list.id).await?;
    assert!(deleted_list.is_none());

    let empty_lists = TodoList::get_all(&pool).await?;
    assert_eq!(empty_lists.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_get_nonexistent_todo_list() -> Result<()> {
    // Arrange
    let pool = setup_test_db().await?;

    // Act: Try to get a list that doesn't exist
    let result = TodoList::get_by_id(&pool, 999).await?;

    // Assert: Should return None, not an error
    assert!(result.is_none());

    Ok(())
}

#[tokio::test]
async fn test_get_all_empty_database() -> Result<()> {
    // Arrange
    let pool = setup_test_db().await?;

    // Get all lists from empty database
    let lists = TodoList::get_all(&pool).await?;

    // Should return empty vector
    assert_eq!(lists.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_todo_list_creation_edge_cases() -> Result<()> {
    let pool = setup_test_db().await?;

    // Test creating list with empty name
    let empty_name_list = NewTodoList {
        name: "".to_string(),
    };
    let created_empty = TodoList::create(&pool, empty_name_list).await?;
    assert_eq!(created_empty.name, "");

    // Test creating list with very long name
    let long_name = "A".repeat(1000);
    let long_name_list = NewTodoList {
        name: long_name.clone(),
    };
    let created_long = TodoList::create(&pool, long_name_list).await?;
    assert_eq!(created_long.name, long_name);

    // Test creating list with special characters
    let special_name = "Special chars: !@#$%^&*()_+-=[]{}|;':\",./<>?`~".to_string();
    let special_list = NewTodoList {
        name: special_name.clone(),
    };
    let created_special = TodoList::create(&pool, special_list).await?;
    assert_eq!(created_special.name, special_name);

    // Test creating list with unicode characters
    let unicode_name = "Unicode: 🚀 测试 🎉 café naïve résumé".to_string();
    let unicode_list = NewTodoList {
        name: unicode_name.clone(),
    };
    let created_unicode = TodoList::create(&pool, unicode_list).await?;
    assert_eq!(created_unicode.name, unicode_name);

    // Test creating list with newlines and tabs
    let multiline_name = "Line 1\nLine 2\tTabbed".to_string();
    let multiline_list = NewTodoList {
        name: multiline_name.clone(),
    };
    let created_multiline = TodoList::create(&pool, multiline_list).await?;
    assert_eq!(created_multiline.name, multiline_name);

    Ok(())
}

#[tokio::test]
async fn test_todo_list_name_update_edge_cases() -> Result<()> {
    let pool = setup_test_db().await?;

    let mut test_list = TodoList::create(
        &pool,
        NewTodoList {
            name: "Original name".to_string(),
        },
    )
    .await?;

    // Update to empty string
    test_list.update_name(&pool, "".to_string()).await?;
    assert_eq!(test_list.name, "");

    // Update to very long name
    let long_name = "Very ".repeat(200) + "long name";
    test_list.update_name(&pool, long_name.clone()).await?;
    assert_eq!(test_list.name, long_name);

    // Update to name with special characters
    let special_name = "Special chars: !@#$%^&*()_+-=[]{}|;':\",./<>?`~".to_string();
    test_list.update_name(&pool, special_name.clone()).await?;
    assert_eq!(test_list.name, special_name);

    // Update to name with unicode characters
    let unicode_name = "Unicode: 🚀 测试 🎉 café naïve résumé".to_string();
    test_list.update_name(&pool, unicode_name.clone()).await?;
    assert_eq!(test_list.name, unicode_name);

    // Update to name with newlines and tabs
    let multiline_name = "Line 1\nLine 2\tTabbed".to_string();
    test_list.update_name(&pool, multiline_name.clone()).await?;
    assert_eq!(test_list.name, multiline_name);

    // Verify final state persists in database
    let fetched_list = TodoList::get_by_id(&pool, test_list.id)
        .await?
        .expect("List should exist");
    assert_eq!(fetched_list.name, multiline_name);

    Ok(())
}

#[tokio::test]
async fn test_todo_list_timestamp_updates() -> Result<()> {
    let pool = setup_test_db().await?;

    let mut test_list = TodoList::create(
        &pool,
        NewTodoList {
            name: "Timestamp test".to_string(),
        },
    )
    .await?;

    let original_created_at = test_list.created_at;
    let original_updated_at = test_list.updated_at;

    // Small delay to ensure timestamp difference
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Update name and verify timestamp changes
    test_list
        .update_name(&pool, "Updated name".to_string())
        .await?;
    assert_eq!(test_list.created_at, original_created_at); // Created should not change
    assert!(test_list.updated_at > original_updated_at); // Updated should change

    Ok(())
}

#[tokio::test]
async fn test_todo_list_cascade_deletion() -> Result<()> {
    let pool = setup_test_db().await?;

    // Create a list
    let test_list = TodoList::create(
        &pool,
        NewTodoList {
            name: "List with items".to_string(),
        },
    )
    .await?;

    // Create multiple items in the list
    let item1 = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: test_list.id,
            name: "Item 1".to_string(),
            priority: Some(Priority::High),
            due_date: Some(Utc::now() + Duration::days(1)),
        },
    )
    .await?;

    let item2 = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: test_list.id,
            name: "Item 2".to_string(),
            priority: Some(Priority::Medium),
            due_date: None,
        },
    )
    .await?;

    let item3 = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: test_list.id.clone(),
            name: "Item 3".to_string(),
            priority: None,
            due_date: Some(Utc::now() - Duration::days(1)),
        },
    )
    .await?;

    // Save the id
    let test_list_id = test_list.id.clone();

    // Verify items exist
    let items_before = TodoItem::get_by_list_id(&pool, test_list_id.clone()).await?;
    assert_eq!(items_before.len(), 3);

    // Delete the list
    test_list.delete(&pool).await?;

    // Verify list is deleted
    let deleted_list = TodoList::get_by_id(&pool, test_list_id.clone()).await?;
    assert!(deleted_list.is_none());

    // Verify all items are cascade deleted
    let items_after = TodoItem::get_by_list_id(&pool, test_list_id.clone()).await?;
    assert_eq!(items_after.len(), 0);

    // Verify each item individually
    assert!(TodoItem::get_by_id(&pool, item1.id).await?.is_none());
    assert!(TodoItem::get_by_id(&pool, item2.id).await?.is_none());
    assert!(TodoItem::get_by_id(&pool, item3.id).await?.is_none());

    Ok(())
}

#[tokio::test]
async fn test_todo_list_ordering() -> Result<()> {
    let pool = setup_test_db().await?;

    // Create lists with small delays to ensure different creation times
    let _list1 = TodoList::create(
        &pool,
        NewTodoList {
            name: "First List".to_string(),
        },
    )
    .await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let _list2 = TodoList::create(
        &pool,
        NewTodoList {
            name: "Second List".to_string(),
        },
    )
    .await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let _list3 = TodoList::create(
        &pool,
        NewTodoList {
            name: "Third List".to_string(),
        },
    )
    .await?;

    // Get all lists - should be ordered by created_at
    let all_lists = TodoList::get_all(&pool).await?;
    assert_eq!(all_lists.len(), 3);

    // Verify ordering
    assert_eq!(all_lists[0].name, "First List");
    assert_eq!(all_lists[1].name, "Second List");
    assert_eq!(all_lists[2].name, "Third List");

    // Verify timestamps are in order
    assert!(all_lists[0].created_at <= all_lists[1].created_at);
    assert!(all_lists[1].created_at <= all_lists[2].created_at);

    Ok(())
}

#[tokio::test]
async fn test_ui_list_operations() -> Result<()> {
    let pool = setup_test_db().await?;

    // Test UIList::get_all() with empty database
    let empty_ui_lists = UIList::get_all(&pool).await?;
    assert_eq!(empty_ui_lists.len(), 0);

    // Create some lists and items
    let list1 = TodoList::create(
        &pool,
        NewTodoList {
            name: "UI Test List 1".to_string(),
        },
    )
    .await?;

    let list2 = TodoList::create(
        &pool,
        NewTodoList {
            name: "UI Test List 2".to_string(),
        },
    )
    .await?;

    // Add items to first list
    let _item1 = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: list1.id,
            name: "Item 1 in List 1".to_string(),
            priority: Some(Priority::High),
            due_date: None,
        },
    )
    .await?;

    let _item2 = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: list1.id,
            name: "Item 2 in List 1".to_string(),
            priority: Some(Priority::Low),
            due_date: Some(Utc::now() + Duration::days(3)),
        },
    )
    .await?;

    // Add one item to second list
    let _item3 = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: list2.id,
            name: "Item 1 in List 2".to_string(),
            priority: Some(Priority::Medium),
            due_date: None,
        },
    )
    .await?;

    // Test UIList::get_all() with data
    let ui_lists = UIList::get_all(&pool).await?;
    assert_eq!(ui_lists.len(), 2);

    // Verify first list
    assert_eq!(ui_lists[0].list.name, "UI Test List 1");
    assert_eq!(ui_lists[0].items.len(), 2);
    assert_eq!(ui_lists[0].items[0].item.name, "Item 1 in List 1");
    assert_eq!(ui_lists[0].items[1].item.name, "Item 2 in List 1");

    // Verify second list
    assert_eq!(ui_lists[1].list.name, "UI Test List 2");
    assert_eq!(ui_lists[1].items.len(), 1);
    assert_eq!(ui_lists[1].items[0].item.name, "Item 1 in List 2");

    Ok(())
}

#[tokio::test]
async fn test_ui_list_update_items() -> Result<()> {
    let pool = setup_test_db().await?;

    // Create a list
    let test_list = TodoList::create(
        &pool,
        NewTodoList {
            name: "Update Test List".to_string(),
        },
    )
    .await?;

    // Create initial items
    let _item1 = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: test_list.id,
            name: "Initial Item".to_string(),
            priority: Some(Priority::High),
            due_date: None,
        },
    )
    .await?;

    // Get initial UI list state
    let mut ui_lists = UIList::get_all(&pool).await?;
    assert_eq!(ui_lists.len(), 1);
    assert_eq!(ui_lists[0].items.len(), 1);

    // Add more items directly to database
    let _item2 = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: test_list.id,
            name: "Added Item 1".to_string(),
            priority: Some(Priority::Medium),
            due_date: Some(Utc::now() + Duration::days(1)),
        },
    )
    .await?;

    let _item3 = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: test_list.id,
            name: "Added Item 2".to_string(),
            priority: Some(Priority::Low),
            due_date: None,
        },
    )
    .await?;

    // Update items using UIList::update_items
    ui_lists[0].update_items(&pool).await?;

    // Verify items were updated
    assert_eq!(ui_lists[0].items.len(), 3);
    assert_eq!(ui_lists[0].items[0].item.name, "Initial Item");
    assert_eq!(ui_lists[0].items[1].item.name, "Added Item 1");
    assert_eq!(ui_lists[0].items[2].item.name, "Added Item 2");

    // Delete an item from database
    ui_lists[0].items[1].item.clone().delete(&pool).await?;

    // Update items again
    ui_lists[0].update_items(&pool).await?;

    // Verify item was removed
    assert_eq!(ui_lists[0].items.len(), 2);
    assert_eq!(ui_lists[0].items[0].item.name, "Initial Item");
    assert_eq!(ui_lists[0].items[1].item.name, "Added Item 2");

    Ok(())
}

#[tokio::test]
async fn test_ui_list_with_empty_lists() -> Result<()> {
    let pool = setup_test_db().await?;

    // Create lists without items
    let _list1 = TodoList::create(
        &pool,
        NewTodoList {
            name: "Empty List 1".to_string(),
        },
    )
    .await?;

    let _list2 = TodoList::create(
        &pool,
        NewTodoList {
            name: "Empty List 2".to_string(),
        },
    )
    .await?;

    // Get UI lists
    let ui_lists = UIList::get_all(&pool).await?;
    assert_eq!(ui_lists.len(), 2);

    // Verify both lists have no items
    assert_eq!(ui_lists[0].items.len(), 0);
    assert_eq!(ui_lists[1].items.len(), 0);
    assert_eq!(ui_lists[0].list.name, "Empty List 1");
    assert_eq!(ui_lists[1].list.name, "Empty List 2");

    Ok(())
}

#[tokio::test]
async fn test_multiple_lists_with_mixed_items() -> Result<()> {
    let pool = setup_test_db().await?;

    // Create multiple lists
    let list1 = TodoList::create(
        &pool,
        NewTodoList {
            name: "Shopping".to_string(),
        },
    )
    .await?;

    let list2 = TodoList::create(
        &pool,
        NewTodoList {
            name: "Work Tasks".to_string(),
        },
    )
    .await?;

    let list3 = TodoList::create(
        &pool,
        NewTodoList {
            name: "Personal".to_string(),
        },
    )
    .await?;

    // Add various items to each list
    // Shopping list
    let _shop1 = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: list1.id,
            name: "Buy milk".to_string(),
            priority: Some(Priority::High),
            due_date: Some(Utc::now() + Duration::hours(2)),
        },
    )
    .await?;

    let _shop2 = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: list1.id,
            name: "Buy bread".to_string(),
            priority: Some(Priority::Medium),
            due_date: None,
        },
    )
    .await?;

    // Work tasks
    let _work1 = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: list2.id,
            name: "Finish report".to_string(),
            priority: Some(Priority::High),
            due_date: Some(Utc::now() + Duration::days(1)),
        },
    )
    .await?;

    let _work2 = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: list2.id,
            name: "Review code".to_string(),
            priority: Some(Priority::Medium),
            due_date: Some(Utc::now() + Duration::days(2)),
        },
    )
    .await?;

    let _work3 = TodoItem::create(
        &pool,
        NewTodoItem {
            list_id: list2.id,
            name: "Team meeting".to_string(),
            priority: Some(Priority::Low),
            due_date: Some(Utc::now() + Duration::days(3)),
        },
    )
    .await?;

    // Personal (no items initially)

    // Verify all lists and their items
    let all_lists = TodoList::get_all(&pool).await?;
    assert_eq!(all_lists.len(), 3);

    let list1_items = TodoItem::get_by_list_id(&pool, list1.id).await?;
    assert_eq!(list1_items.len(), 2);

    let list2_items = TodoItem::get_by_list_id(&pool, list2.id).await?;
    assert_eq!(list2_items.len(), 3);

    let list3_items = TodoItem::get_by_list_id(&pool, list3.id).await?;
    assert_eq!(list3_items.len(), 0);

    // Test UI lists
    let ui_lists = UIList::get_all(&pool).await?;
    assert_eq!(ui_lists.len(), 3);
    assert_eq!(ui_lists[0].items.len(), 2); // Shopping
    assert_eq!(ui_lists[1].items.len(), 3); // Work
    assert_eq!(ui_lists[2].items.len(), 0); // Personal

    Ok(())
}
