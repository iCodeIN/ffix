use crate::{Error, Result};

pub struct ArrayReader<T> {
    ptr: *mut *mut T,
    curr_idx: isize,
    max_len: isize,
}

impl<T> ArrayReader<T> {
    pub fn new(ptr: *mut *mut T, max_len: isize) -> Result<Self> {
        if ptr.is_null() {
            Err(Error::Null)
        } else {
            Ok(Self {
                ptr,
                max_len,
                curr_idx: 0,
            })
        }
    }
}

impl<T> Iterator for ArrayReader<T> {
    type Item = *mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr.is_null() || self.curr_idx == self.max_len {
            return None;
        }
        let item_ptr = unsafe { *self.ptr.offset(self.curr_idx) };
        if item_ptr.is_null() {
            None
        } else {
            self.curr_idx += 1;
            Some(item_ptr)
        }
    }
}
