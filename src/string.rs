use crate::{Error, Result};
use libc::{c_char, c_void, malloc, memcpy, memset};
use std::{
    ffi::{CStr, CString},
    mem,
};

#[doc(hidden)]
pub struct StringReader {
    buf: Vec<i8>,
}

impl StringReader {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buf: Vec::with_capacity(capacity),
        }
    }

    pub fn as_mut_ptr(&mut self) -> *mut c_char {
        self.buf.as_mut_ptr()
    }

    pub fn into_string(self) -> Result<String> {
        self.into_string_opt()?.ok_or_else(|| Error::Null)
    }

    pub fn into_string_opt(mut self) -> Result<Option<String>> {
        let ptr = self.buf.as_mut_ptr();
        mem::forget(self.buf);
        if ptr.is_null() {
            Ok(None)
        } else {
            Ok(Some(unsafe { CString::from_raw(ptr) }.into_string()?))
        }
    }
}

#[doc(hidden)]
pub struct StringArrayReader {
    ptr: *mut *const c_char,
    curr_idx: isize,
    max_len: isize,
}

impl StringArrayReader {
    pub fn new(ptr: *mut *const c_char, max_len: isize) -> Self {
        Self {
            ptr,
            max_len,
            curr_idx: 0,
        }
    }
}

impl Iterator for StringArrayReader {
    type Item = Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr.is_null() || self.curr_idx == self.max_len {
            return None;
        }
        let item_ptr = unsafe { *self.ptr.offset(self.curr_idx) };
        if item_ptr.is_null() {
            None
        } else {
            self.curr_idx += 1;
            Some(
                unsafe { CStr::from_ptr(item_ptr) }
                    .to_str()
                    .map(|x| x.to_string())
                    .map_err(Error::from),
            )
        }
    }
}

#[doc(hidden)]
pub trait ToCStringArray<T> {
    fn to_c_string_array(items: T) -> Result<*mut *const i8>;
}

impl<T, I> ToCStringArray<T> for T
where
    T: IntoIterator<Item = I>,
    I: AsRef<str>,
{
    fn to_c_string_array(items: T) -> Result<*mut *const i8> {
        let items: Vec<I> = items.into_iter().collect();
        let array_size = mem::size_of::<*const i8>() * (items.len() + 1);
        let array_ptr = unsafe {
            let ptr = malloc(array_size);
            assert!(!ptr.is_null());
            memset(ptr, 0, array_size);
            ptr as *mut *const i8
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
        Ok(array_ptr)
    }
}

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
