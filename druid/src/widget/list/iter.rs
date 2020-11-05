#[cfg(feature = "im")]
use crate::im::Vector;
use crate::Data;
use std::sync::Arc;

/// This iterator enables writing List widget for any `Data`.
pub trait ListIter<T>: Data {
    /// Iterate over each data child.
    fn for_each(&self, cb: impl FnMut(&T, usize));

    /// Iterate over each data child. Keep track of changed data and update self.
    fn for_each_mut(&mut self, cb: impl FnMut(&mut T, usize));

    /// Return data length.
    fn data_len(&self) -> usize;
}

#[cfg(feature = "im")]
impl<T: Data> ListIter<T> for Vector<T> {
    fn for_each(&self, mut cb: impl FnMut(&T, usize)) {
        for (i, item) in self.iter().enumerate() {
            cb(item, i);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut T, usize)) {
        for (i, item) in self.iter_mut().enumerate() {
            cb(item, i);
        }
    }

    fn data_len(&self) -> usize {
        self.len()
    }
}

// S == shared data type
#[cfg(feature = "im")]
impl<S: Data, T: Data> ListIter<(S, T)> for (S, Vector<T>) {
    fn for_each(&self, mut cb: impl FnMut(&(S, T), usize)) {
        for (i, item) in self.1.iter().enumerate() {
            let d = (self.0.to_owned(), item.to_owned());
            cb(&d, i);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut (S, T), usize)) {
        for (i, item) in self.1.iter_mut().enumerate() {
            let mut d = (self.0.clone(), item.clone());
            cb(&mut d, i);

            if !self.0.same(&d.0) {
                self.0 = d.0;
            }
            if !item.same(&d.1) {
                *item = d.1;
            }
        }
    }

    fn data_len(&self) -> usize {
        self.1.len()
    }
}

impl<T: Data> ListIter<T> for Arc<Vec<T>> {
    fn for_each(&self, mut cb: impl FnMut(&T, usize)) {
        for (i, item) in self.iter().enumerate() {
            cb(item, i);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut T, usize)) {
        let mut new_data = Vec::with_capacity(self.data_len());
        let mut any_changed = false;

        for (i, item) in self.iter().enumerate() {
            let mut d = item.to_owned();
            cb(&mut d, i);

            if !any_changed && !item.same(&d) {
                any_changed = true;
            }
            new_data.push(d);
        }

        if any_changed {
            *self = Arc::new(new_data);
        }
    }

    fn data_len(&self) -> usize {
        self.len()
    }
}

// S == shared data type
impl<S: Data, T: Data> ListIter<(S, T)> for (S, Arc<Vec<T>>) {
    fn for_each(&self, mut cb: impl FnMut(&(S, T), usize)) {
        for (i, item) in self.1.iter().enumerate() {
            let d = (self.0.clone(), item.to_owned());
            cb(&d, i);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut (S, T), usize)) {
        let mut new_data = Vec::with_capacity(self.1.len());
        let mut any_shared_changed = false;
        let mut any_el_changed = false;

        for (i, item) in self.1.iter().enumerate() {
            let mut d = (self.0.clone(), item.to_owned());
            cb(&mut d, i);

            if !any_shared_changed && !self.0.same(&d.0) {
                any_shared_changed = true;
            }
            if any_shared_changed {
                self.0 = d.0;
            }
            if !any_el_changed && !item.same(&d.1) {
                any_el_changed = true;
            }
            new_data.push(d.1);
        }

        if any_el_changed {
            self.1 = Arc::new(new_data);
        }
    }

    fn data_len(&self) -> usize {
        self.1.len()
    }
}
