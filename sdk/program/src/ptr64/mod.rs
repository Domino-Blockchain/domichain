use std::marker::PhantomData;

pub struct Ptr64<T: ?Sized> {
    address: u64,
    phantom_data: PhantomData<*mut T>,
}

impl<T: ?Sized> From<*mut T> for Ptr64<T> {
    fn from(value: *mut T) -> Self {
        Self { address: value as *const () as u64, phantom_data: PhantomData }
    }
}
impl<T: ?Sized> From<*const T> for Ptr64<T> {
    fn from(value: *const T) -> Self {
        Self { address: value as *const () as u64, phantom_data: PhantomData }
    }
}

pub struct Usize64(u64);

impl From<usize> for Usize64 {
    fn from(value: usize) -> Self {
        Self(value as u64)
    }
}


pub struct FatPtr64<T> {
    ptr: Ptr64<[T]>,
    len: Usize64,
}

impl<'a, T> From<&'a mut [T]> for FatPtr64<T> {
    fn from(value: &'a mut [T]) -> Self {
        let ptr = value as *mut [T];
        let len = value.len();
        Self { ptr: ptr.into(), len: len.into() }
    }
}
impl<'a, T> From<&'a [T]> for FatPtr64<T> {
    fn from(value: &'a [T]) -> Self {
        let ptr = value as *const [T];
        let len = value.len();
        Self { ptr: ptr.into(), len: len.into() }
    }
}

pub fn convert_nested_slice<T>(s: &[&[T]]) -> Vec<FatPtr64<T>> {
    s.iter().map(|ss| FatPtr64::from(*ss)).collect()
}