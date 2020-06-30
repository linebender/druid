use druid::{Data, Lens};

#[rustfmt::skip]
mod todo_item_lens {
use druid::Data;
// ANCHOR: todo_item
/// A single todo item.
#[derive(Clone, Data)]
struct TodoItem {
    title: String,
    completed: bool,
    urgent: bool,
}
// ANCHOR_END: todo_item
}

// ANCHOR: todo_item_lens
/// A single todo item.
#[derive(Clone, Data, Lens)]
struct TodoItem {
    title: String,
    completed: bool,
    urgent: bool,
}
// ANCHOR_END: todo_item_lens

// ANCHOR: simple_lens
trait SimpleLens<In, Out> {
    fn focus(&self, data: &In) -> Out;
}
// ANCHOR_END: simple_lens

// ANCHOR: completed_lens
/// This is the type of the lens itself; in this case it has no state.
struct CompletedLens;

impl SimpleLens<TodoItem, bool> for CompletedLens {
    fn focus(&self, data: &TodoItem) -> bool {
        data.completed
    }
}
// ANCHOR_END: completed_lens

#[rustfmt::skip]
mod lens_impl {
// ANCHOR: lens
pub trait Lens<In, Out> {
    /// Get non-mut access to the field.
    fn with<R, F: FnOnce(&Out) -> R>(&self, data: &In, f: F) -> R;
    /// Get mut access to the field.
    fn with_mut<R, F: FnOnce(&mut Out) -> R>(&self, data: &mut In, f: F) -> R;
}
// ANCHOR_END: lens

use super::TodoItem;
// ANCHOR: completed_lens_real
struct CompletedLens;

impl Lens<TodoItem, bool> for CompletedLens {
    fn with<R, F: FnOnce(&bool) -> R>(&self, data: &TodoItem, f: F) -> R {
        f(&data.completed)
    }

    fn with_mut<R, F: FnOnce(&mut bool) -> R>(&self, data: &mut TodoItem, f: F) -> R {
        f(&mut data.completed)
    }
}
// ANCHOR_END: completed_lens_real
}

//
// ANCHOR: lens_name
#[derive(Lens)]
struct Item {
    #[lens(name = "count_lens")]
    count: usize,
}

// This works now:
impl Item {
    fn count(&self) -> usize {
        self.count
    }
}
// ANCHOR_END: lens_name

// ANCHOR: build_ui
use druid::widget::{Checkbox, Flex, Label, Widget, WidgetExt};

fn make_todo_item() -> impl Widget<TodoItem> {
    // A label that generates its text based on the data:
    let title = Label::dynamic(|text: &String, _| text.to_string()).lens(TodoItem::title);
    let completed = Checkbox::new("Completed:").lens(TodoItem::completed);
    let urgent = Checkbox::new("Urgent:").lens(TodoItem::urgent);

    Flex::column()
        // label on top
        .with_child(title)
        // two checkboxes below
        .with_child(Flex::row().with_child(completed).with_child(urgent))
}
// ANCHOR_END: build_ui

use std::collections::HashMap;
use std::sync::Arc;
// ANCHOR: contact
#[derive(Clone, Data)]
struct Contact {
    // fields
}

type ContactId = u64;

#[derive(Clone, Data)]
struct Contacts {
    inner: Arc<HashMap<ContactId, Contact>>,
}

// Lets write a lens that returns a specific contact based on its id, if it exists.

struct ContactIdLens(ContactId);

impl Lens<Contacts, Option<Contact>> for ContactIdLens {
    fn with<R, F: FnOnce(&Option<Contact>) -> R>(&self, data: &Contacts, f: F) -> R {
        let contact = data.inner.get(&self.0).cloned();
        f(&contact)
    }

    fn with_mut<R, F: FnOnce(&mut Option<Contact>) -> R>(&self, data: &mut Contacts, f: F) -> R {
        // get an immutable copy
        let mut contact = data.inner.get(&self.0).cloned();
        let result = f(&mut contact);
        // only actually mutate the collection if our result is mutated;
        let changed = match (contact.as_ref(), data.inner.get(&self.0)) {
            (Some(one), Some(two)) => !one.same(two),
            (None, None) => false,
            _ => true,
        };
        if changed {
            // if !data.inner.get(&self.0).same(&contact.as_ref()) {
            let contacts = Arc::make_mut(&mut data.inner);
            // if we're none, we were deleted, and remove from the map; else replace
            match contact {
                Some(contact) => contacts.insert(self.0, contact),
                None => contacts.remove(&self.0),
            };
        }
        result
    }
}
// ANCHOR_END: contact

// ANCHOR: conversion
struct MilesToKm;

const KM_PER_MILE: f64 = 1.609_344;

impl Lens<f64, f64> for MilesToKm {
    fn with<R, F: FnOnce(&f64) -> R>(&self, data: &f64, f: F) -> R {
        let kms = *data * KM_PER_MILE;
        f(&kms)
    }

    fn with_mut<R, F: FnOnce(&mut f64) -> R>(&self, data: &mut f64, f: F) -> R {
        let mut kms = *data * KM_PER_MILE;
        let kms_2 = kms;
        let result = f(&mut kms);
        // avoid doing the conversion if unchanged, it might be lossy?
        if !kms.same(&kms_2) {
            let miles = kms * KM_PER_MILE.recip();
            *data = miles;
        }
        result
    }
}
// ANCHOR_END: conversion
