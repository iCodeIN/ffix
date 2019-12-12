use crate::{Error, Result};
use libc::{c_char, c_void, free, malloc, memset};
use std::{
    ffi::{CStr, CString},
    mem,
    ptr::NonNull,
};

/// A helper to read C string
pub struct StringReader {
    buf: Vec<i8>,
}

impl StringReader {
    /// Create a new reader
    ///
    /// # Arguments
    ///
    /// * max_length - Maximum length of string
    pub fn new(max_length: usize) -> Self {
        Self {
            buf: Vec::with_capacity(max_length),
        }
    }

    /// Get a pointer to read to
    pub fn get_target(&mut self) -> *mut c_char {
        self.buf.as_mut_ptr()
    }

    /// Get a result string
    pub fn into_string(self) -> Result<String> {
        self.into_string_opt()?.ok_or_else(|| Error::Null)
    }

    /// Get a result string or None if pointer is NULL
    pub fn into_string_opt(mut self) -> Result<Option<String>> {
        let ptr = self.buf.as_mut_ptr();
        mem::forget(self.buf);
        if ptr.is_null() {
            Ok(None)
        } else {
            Ok(Some(
                unsafe { CStr::from_ptr(ptr) }
                    .to_str()
                    .map(|x| x.to_string())?,
            ))
        }
    }
}

/// A wrapper for null-terminated C string array
pub struct StringArray {
    ptr: NonNull<*const c_char>,
    should_drop: bool,
    has_dropped: bool,
}

impl StringArray {
    /// Creates a new string array
    ///
    /// # Panics
    ///
    /// Panics if memory allocation failed
    ///
    /// # Arguments
    ///
    /// * items - Items to copy
    pub fn new<T, I>(items: T) -> Result<Self>
    where
        T: IntoIterator<Item = I>,
        I: AsRef<str>,
    {
        let items: Vec<I> = items.into_iter().collect();
        let array_size = mem::size_of::<*const c_char>() * (items.len() + 1);
        let array_ptr = unsafe {
            let ptr = malloc(array_size);
            assert!(!ptr.is_null());
            memset(ptr, 0, array_size);
            ptr as *mut *const c_char
        };
        for (item_idx, item_data) in items.iter().enumerate() {
            let item_idx = item_idx as isize;
            let item_data = item_data.as_ref().as_bytes();
            unsafe {
                let item_ptr = array_ptr.offset(item_idx);
                *item_ptr = expose_string(item_data)?;
            }
        }
        Ok(Self {
            ptr: unsafe { NonNull::new_unchecked(array_ptr) },
            should_drop: true,
            has_dropped: false,
        })
    }

    /// Returns a raw pointer to string array
    ///
    /// You MUST be sure that string array is deallocated
    ///
    /// Use `from_raw` method with `sould_drop=true`,
    /// or make sure that C code deallocates a returned data.
    pub fn into_raw(mut self) -> *mut *const c_char {
        self.should_drop = false;
        self.ptr.as_ptr()
    }

    /// Constructs a string array from raw pointer
    ///
    /// # Safety
    ///
    /// Improper use may lead to memory problems.
    /// For example, a double-free may occur
    /// if the function is called twice on the same raw pointer.
    ///
    /// # Panics
    ///
    /// Pointer must be not NULL
    ///
    /// # Arguments
    ///
    /// * ptr - A pointer to C string array
    /// * should_drop - Should data be deallocated when `drop()` is called
    pub unsafe fn from_raw(ptr: *mut *const c_char, should_drop: bool) -> Self {
        Self {
            ptr: NonNull::new(ptr).expect("Pointer must be not NULL"),
            should_drop,
            has_dropped: false,
        }
    }

    fn free(&mut self) {
        if self.should_drop && !self.has_dropped {
            unsafe { free(self.ptr.as_ptr().cast()) }
            self.has_dropped = true;
        }
    }
}

impl Drop for StringArray {
    fn drop(&mut self) {
        self.free()
    }
}

impl IntoIterator for StringArray {
    type Item = Result<String>;
    type IntoIter = StringArrayIter;

    fn into_iter(self) -> Self::IntoIter {
        StringArrayIter::new(self)
    }
}

/// Iterator over StringArray
pub struct StringArrayIter {
    array: StringArray,
    current_index: isize,
}

impl StringArrayIter {
    fn new(array: StringArray) -> Self {
        Self {
            array,
            current_index: 0,
        }
    }
}

impl Iterator for StringArrayIter {
    type Item = Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        let item_ptr = unsafe { *self.array.ptr.as_ptr().offset(self.current_index) };
        if item_ptr.is_null() {
            None
        } else {
            self.current_index += 1;
            Some(
                unsafe { CStr::from_ptr(item_ptr) }
                    .to_str()
                    .map(|x| x.to_string())
                    .map_err(Error::from),
            )
        }
    }
}

/// Copies a rust string to a newly allocated C String
///
/// Use this function if you are unable to deallocate string in Rust code.
/// You MUST be sure that string is deallocated.
pub fn expose_string<T: Into<Vec<u8>>>(input: T) -> Result<*const c_char> {
    let input = input.into();
    let size = input.len() + 1;
    let input = CString::new(input)?;
    let src = input.as_ptr().cast::<c_void>();
    let dest = unsafe {
        let p = malloc(size);
        assert!(!p.is_null());
        src.copy_to_nonoverlapping(p, size);
        p.cast::<i8>()
    };
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use libc::strcpy;
    use std::ptr::null_mut;

    #[test]
    fn test_read_and_write_string() {
        let input = "test";
        let ptr = expose_string(input).unwrap();
        let mut reader = StringReader::new(input.len());
        let copy_ptr = reader.get_target();
        unsafe { strcpy(copy_ptr, ptr) };
        let result = reader.into_string().unwrap();
        assert_eq!(result, input);
        unsafe { libc::free(ptr as *mut c_void) };
    }

    #[test]
    fn test_read_and_write_string_array() {
        let array = StringArray::new(&["a", "b", "c"]).unwrap();
        let ptr = array.into_raw();
        let array = unsafe { StringArray::from_raw(ptr, true) };
        let items: Vec<String> = array.into_iter().map(|x| x.unwrap()).collect();
        assert_eq!(items, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_drop_string_array() {
        let array = StringArray::new(&["a", "b", "c"]).unwrap();
        let ptr = array.into_raw();
        let mut array = unsafe { StringArray::from_raw(ptr, false) };
        array.free();
        assert!(!array.has_dropped);
        let mut array = unsafe { StringArray::from_raw(ptr, true) };
        array.free();
        assert!(array.has_dropped);
    }

    #[test]
    #[should_panic]
    fn test_string_array_from_raw_null() {
        let _ = unsafe { StringArray::from_raw(null_mut(), true) };
    }
}
