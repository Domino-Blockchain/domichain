use std::marker::PhantomData;

#[derive(Debug)]
pub struct Ptr64<T: ?Sized> {
    pub address: u64,
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

#[derive(Debug)]
pub struct Usize64(u64);

impl From<usize> for Usize64 {
    fn from(value: usize) -> Self {
        Self(value as u64)
    }
}

#[derive(Debug)]
pub struct WidePtr64<T> {
    pub ptr: Ptr64<[T]>,
    pub len: Usize64,
}

impl<'a, T> From<&'a mut [T]> for WidePtr64<T> {
    fn from(value: &'a mut [T]) -> Self {
        let ptr = value as *mut [T];
        let len = value.len();
        Self { ptr: ptr.into(), len: len.into() }
    }
}
impl<'a, T> From<&'a [T]> for WidePtr64<T> {
    fn from(value: &'a [T]) -> Self {
        let ptr = value as *const [T];
        let len = value.len();
        Self { ptr: ptr.into(), len: len.into() }
    }
}

pub fn convert_nested_slice<T>(s: &[&[T]]) -> Vec<WidePtr64<T>> {
    s.iter().map(|ss| WidePtr64::from(*ss)).collect()
}
