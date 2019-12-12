use std::ptr::NonNull;

/// Null-terminated C array reader
pub struct ArrayReader<T> {
    ptr: NonNull<*mut T>,
}

impl<T> ArrayReader<T> {
    /// Create a new array from raw pointer
    ///
    /// # Safety
    ///
    /// * Array must be always non-null
    /// * Array must be terminated with NULL
    ///
    /// See also [ptr::add](https://doc.rust-lang.org/stable/std/primitive.pointer.html?search=#method.add)
    ///
    /// # Panics
    ///
    /// Panics if pointer is NULL
    ///
    /// # Arguments
    ///
    /// * ptr - Pointer to array
    pub unsafe fn new(ptr: *mut *mut T) -> Self {
        Self {
            ptr: NonNull::new(ptr).expect("Pointer must be not null"),
        }
    }

    /// Get an item from array by index
    ///
    /// # Panics
    ///
    /// Panics if array pointer is NULL
    ///
    /// # Arguments
    ///
    /// * index - Index of an item
    pub fn get(&self, index: usize) -> Option<*mut T> {
        let ptr = unsafe { *self.ptr.as_ptr().add(index) };
        if ptr.is_null() {
            None
        } else {
            Some(ptr)
        }
    }
}

impl<T> IntoIterator for ArrayReader<T> {
    type Item = *mut T;
    type IntoIter = ArrayIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        ArrayIter::new(self)
    }
}

/// Iterator over a C array reader
pub struct ArrayIter<T> {
    reader: ArrayReader<T>,
    current_index: usize,
}

impl<T> ArrayIter<T> {
    fn new(reader: ArrayReader<T>) -> Self {
        Self {
            reader,
            current_index: 0,
        }
    }
}

impl<T> Iterator for ArrayIter<T> {
    type Item = *mut T;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.reader.get(self.current_index);
        self.current_index += 1;
        item
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libc::{free, malloc};
    use std::{mem, ptr::null_mut};

    #[repr(C)]
    struct Item {
        value: usize,
    }

    struct Array {
        ptr: *mut *mut Item,
    }

    impl Array {
        fn new(length: usize) -> Self {
            let item_size = mem::size_of::<*const Item>();
            let array_size = item_size * (length + 1);
            let array_ptr = unsafe {
                let ptr = malloc(array_size);
                assert!(!ptr.is_null());
                ptr as *mut *mut Item
            };
            for i in 0..length {
                unsafe {
                    let ptr = malloc(item_size);
                    assert!(!ptr.is_null());
                    let ptr = ptr.cast::<Item>();
                    (*ptr).value = i;
                    *array_ptr.add(i) = ptr;
                };
            }
            Self { ptr: array_ptr }
        }
    }

    impl Drop for Array {
        fn drop(&mut self) {
            unsafe { free(self.ptr.cast()) }
        }
    }

    #[test]
    fn it_works() {
        let length = 10;
        let array = Array::new(length);
        let reader = unsafe { ArrayReader::new(array.ptr) };
        for i in 0..length {
            let item = reader.get(i).unwrap();
            assert_eq!(unsafe { (*item).value }, i)
        }
        assert!(reader.get(length * 10).is_none());
        let values: Vec<usize> = reader.into_iter().map(|x| unsafe { (*x).value }).collect();
        assert_eq!(values, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    #[should_panic]
    fn create_from_null() {
        let _: ArrayReader<Item> = unsafe { ArrayReader::new(null_mut()) };
    }
}
