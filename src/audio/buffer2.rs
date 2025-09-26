use std::ops::{Deref, DerefMut, Index, IndexMut};

pub trait Sample: Default + Copy {}
impl Sample for f32 {}
impl Sample for i32 {}
impl Sample for i16 {}

pub struct Buffer<T: Sample> {
    pub sample_rate: u32,
    pub bit_depth: u16,
    pub data: Vec<T>,
}

impl<T: Sample> Buffer<T> {
    pub fn new(sample_rate: u32, bit_depth: u16) -> Self {
        Self {
            sample_rate,
            bit_depth,
            data: vec![],
        }
    }

    pub fn with_capacity(sample_rate: u32, bit_depth: u16, capacity: usize) -> Self {
        Self {
            sample_rate,
            bit_depth,
            data: Vec::with_capacity(capacity),
        }
    }

    pub fn with_size(sample_rate: u32, bit_depth: u16, size: usize) -> Self {
        let mut result = Self {
            sample_rate,
            bit_depth,
            data: Vec::with_capacity(size),
        };
        result.data.resize(size, T::default());
        result
    }

    pub fn duration_s(&self) -> f64 {
        self.data.len() as f64 / self.sample_rate as f64
    }
}


// Use deref to access the underlying buffer
impl<T: Sample> Deref for Buffer<T> {
    type Target = Vec<T>;
    
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl<T: Sample> DerefMut for Buffer<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}


// Indexing support
impl<T: Sample> Index<usize> for Buffer<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl<T: Sample> IndexMut<usize> for Buffer<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}

// Iterator support
impl<T: Sample> IntoIterator for Buffer<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

impl<'a, T: Sample> IntoIterator for &'a Buffer<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

impl<'a, T: Sample> IntoIterator for &'a mut Buffer<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter_mut()
    }
}
