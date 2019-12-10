use crate::{Error, Result};
use libc::{c_char, c_void, free, malloc, memcpy, memset};
use std::{
    ffi::{CStr, CString},
    mem,
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
    ptr: *mut *const c_char,
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
            let item_size = item_data.len();
            let ptr_src = item_data.as_ptr().cast::<c_void>();
            unsafe {
                let ptr_size = item_size + 1;
                let ptr_dest = malloc(ptr_size);
                assert!(!ptr_dest.is_null());
                memset(ptr_dest, 0, ptr_size);
                memcpy(ptr_dest, ptr_src, item_size);
                let item_ptr = array_ptr.offset(item_idx);
                *item_ptr = ptr_dest.cast::<i8>();
            }
        }
        Ok(Self {
            ptr: array_ptr,
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
        self.ptr
    }

    /// Constructs a string array from raw pointer
    ///
    /// # Arguments
    ///
    /// * ptr - A pointer to C string array
    /// * should_drop - Should data be deallocated when `drop()` is called
    pub unsafe fn from_raw(ptr: *mut *const c_char, should_drop: bool) -> Self {
        Self {
            ptr,
            should_drop,
            has_dropped: false,
        }
    }

    fn free(&mut self) {
        if self.should_drop && !self.has_dropped {
            unsafe { free(self.ptr.cast()) }
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
        if self.array.ptr.is_null() {
            return None;
        }
        let item_ptr = unsafe { *self.array.ptr.offset(self.current_index) };
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
}
