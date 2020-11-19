#![allow(clippy::clippy::rc_buffer)]

#[derive(Clone, PartialEq)]
struct DateTime(std::time::Instant);

// ANCHOR: derive
use druid::Data;
use std::sync::Arc;

#[derive(Clone, Data)]
/// The main model for a todo list application.
struct TodoList {
    items: Arc<Vec<TodoItem>>,
}

#[derive(Clone, Data)]
/// A single todo item.
struct TodoItem {
    category: Category,
    // `Data` is implemented for any `Arc`.
    due_date: Option<Arc<DateTime>>,
    // You can specify a custom comparison fn
    // (anything with the signature (&T, &T) -> bool).
    #[data(same_fn = "PartialEq::eq")]
    added_date: DateTime,
    title: String,
    note: Option<String>,
    completed: bool,
}

#[derive(Clone, Data, PartialEq)]
/// The three types of tasks in the world.
enum Category {
    Work,
    Play,
    Revolution,
}
// ANCHOR_END: derive
