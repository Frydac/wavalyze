pub mod id;
pub mod layout;

pub use super::channel::id::Id;
pub use super::channel::layout::Layout;

use super::sample::Sample;
use std::ops::{Deref, DerefMut, Index, IndexMut};

// TODO: replace
// * a channel I would say is a Buffer(id) with a ChannelId?

// Represents one audio channel, i.e. a list of samples
// probably overkill, but I suspect I'm going to want to store some extra metadat in here
#[derive(Debug, Clone)]
pub struct Channel<T: Sample> {
    data: Vec<T>,
}

impl<T: Sample> Channel<T> {
    // reserve empty channel/buffer
    pub fn new(len: usize) -> Self {
        Self {
            data: vec![T::default(); len],
        }
    }
    pub fn samples(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }
    pub fn samples_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.data.iter_mut()
    }

    pub fn at(&self, index: i64) -> Option<&T> {
        if index < 0 {
            None
        } else {
            self.data.get(index as usize)
        }
    }
}

impl<T: Sample> Index<usize> for Channel<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl<T: Sample> IndexMut<usize> for Channel<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}

impl<T: Sample> Deref for Channel<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T: Sample> DerefMut for Channel<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

// This moves the elements out of the channel when looping over it (I think)
// maybe this is not useful? when using movable elements it might be interesting
impl<T: Sample> IntoIterator for Channel<T> {
    type Item = T;

    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

impl<'a, T: Sample> IntoIterator for &'a Channel<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

impl<'a, T: Sample> IntoIterator for &'a mut Channel<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter_mut()
    }
}

// impl<T: Sample> IntoIterato

use std::fmt;

// used when normal println!("{:<width=nr_elements_to_print>.<precision float elements>}", channel)

impl<T: Sample + std::fmt::Display> fmt::Display for Channel<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Determine the number of elements to display
        let max_items = f.width().unwrap_or(self.data.len());
        let max_items = max_items.min(self.data.len());

        let precision = f.precision();

        let mut ds = f.debug_struct("Channel");
        for (i, s) in self.data.iter().enumerate().take(max_items) {
            let formatted_sample = match precision {
                Some(precision) => format!("{:.precision$}", s, precision = precision),
                None => format!("{}", s),
            };
            ds.field(&i.to_string(), &formatted_sample);
        }

        if max_items < self.data.len() {
            ds.field("...", &format!("and {} more", self.data.len() - max_items));
        }

        ds.finish()
    }
}
mod tests {
    #[test]
    fn test_channel_tryouts() {
        let mut ch = super::Channel::<f32>::new(50);
        dbg!(ch.len()); // len is Vec<T>::len due to Deref
        ch[0] = 1.0;
        dbg!(&ch);

        let ch2 = ch.clone();
        ch[1] = 2.561;
        dbg!(&ch);
        dbg!(&ch2);

        // IntoIterator enables looping directly over the class
        // for s in &ch {
        //     dbg!(s);
        // }

        // let width = 10;
        println!("{:5.2}", ch);

        // test display
    }
}
