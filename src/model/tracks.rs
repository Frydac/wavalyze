use crate::{audio, model, rect};
use anyhow::anyhow;
use egui::ahash::HashMap;
use std::ops::Deref;

use super::SampleIx;
// use anyhow::Result;

pub type TrackId = crate::util::Id;

// We want to be able to refer to tracks by id, and they have to be in a specific order (i.e. the
// order of the tracks in the gui)
#[derive(Default, Debug)]
pub struct Tracks {
    pub tracks: HashMap<TrackId, model::track::Track>,
    pub track_order: Vec<u64>,
    // store settings for all tracks:
    // * zoom level x direction
    // * track x position (each track maps an audio buffer to its x position)
    pub samples_per_pixel: Option<f32>,
}

impl Tracks {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, mut track: model::track::Track) {
        // make sure track has the same zoom level
        if let Some(spp) = self.samples_per_pixel {
            track.set_samples_per_pixel(spp);
        }
        self.track_order.push(track.id());
        self.tracks.insert(track.id(), track);
    }

    // insert?

    pub fn track(&self, id: u64) -> Option<&model::track::Track> {
        self.tracks.get(&id)
    }

    pub fn track_mut(&mut self, id: u64) -> Option<&mut model::track::Track> {
        self.tracks.get_mut(&id)
    }

    pub fn track_order(&self) -> &Vec<u64> {
        &self.track_order
    }

    pub fn move_track(&mut self, id: u64, new_index: usize) {
        let index = self.track_order.iter().position(|x| *x == id).unwrap();
        self.track_order.remove(index);
        self.track_order.insert(new_index, id);
    }

    // TODO: more methods to reorder tracks
    // * swap?
    // * rotate? multiple tracks?

    pub fn remove_track(&mut self, id: u64) {
        self.tracks.remove(&id);
        self.track_order.retain(|x| *x != id);
    }

    // Iterate over the tracks in the track_order
    pub fn iter(&self) -> TracksIter<'_> {
        TracksIter {
            map: &self.tracks,
            order: self.track_order.iter(),
        }
    }

    pub fn iter_mut(&mut self) -> TracksIterMut<'_> {
        TracksIterMut {
            map: &mut self.tracks as *mut _,
            order: self.track_order.iter(),
            marker: std::marker::PhantomData,
        }
    }
}

impl Tracks {
    pub fn set_screen_rect(&mut self, id: TrackId, screen_rect: rect::Rect) -> anyhow::Result<()> {
        let track = self.track_mut(id).ok_or(anyhow!(format!("Track with id {} not found", id)))?;

        // If the zoom level hasn't been set in a track, this call will trigger setting default
        // zoom: the track needs to screen width to calculate the view buffer
        track.set_screen_rect(screen_rect)?;

        // We expect this to be true
        if let Some(spp) = track.samples_per_pixel() {
            // If this is the first time set_view_rect is called for any track, we get the
            // 'default' zoom level (zoom level that shows whole buffer on given screen width),
            // from that track and store it
            if self.samples_per_pixel.is_none() {
                self.samples_per_pixel = Some(spp);

                // Initialize all other tracks with the same zoom level
                for track in self.tracks.iter_mut() {
                    track.1.set_samples_per_pixel(spp);
                }
            }
        }
        Ok(())
    }

    pub fn set_sample_rect(&mut self, id: TrackId, sample_rect: audio::SampleRect) -> anyhow::Result<()> {
        let track = self.track_mut(id).ok_or(anyhow!(format!("Track with id {} not found", id)))?;

        // if let Some(track) = self.track_mut(id) {

        // }
        Ok(())
    }
}

impl Tracks {
    // probably want to add some kind of y coordinate for selecting a rectangle
    pub fn select_start(&mut self, id: TrackId, sample: SampleIx) -> anyhow::Result<()> {
        todo!()
    }
    pub fn select_end(&mut self, id: TrackId, sample: SampleIx) -> anyhow::Result<()> {
        todo!()
    }

    // hover over a sample, state is retained until unhover is called
    pub fn hover(&mut self, id: TrackId, sample: SampleIx) {
        todo!()
    }
    pub fn unhover(&mut self, id: TrackId) {
        todo!()
    }
}
impl Deref for Tracks {
    type Target = HashMap<TrackId, model::track::Track>;
    fn deref(&self) -> &Self::Target {
        &self.tracks
    }
}

pub struct TracksIter<'a> {
    map: &'a HashMap<TrackId, model::track::Track>,
    order: std::slice::Iter<'a, u64>,
}

impl<'a> Iterator for TracksIter<'a> {
    type Item = &'a model::track::Track;
    fn next(&mut self) -> Option<Self::Item> {
        self.order.next().map(|id| self.map.get(id).unwrap())
    }
}

pub struct TracksIterMut<'a> {
    map: *mut HashMap<TrackId, model::track::Track>, // raw pointer to the map
    order: std::slice::Iter<'a, u64>,
    marker: std::marker::PhantomData<&'a mut model::track::Track>,
}

impl<'a> Iterator for TracksIterMut<'a> {
    type Item = &'a mut model::track::Track;

    fn next(&mut self) -> Option<Self::Item> {
        // use std::collections::hash_map::Entry;

        let id = self.order.next()?;
        unsafe {
            // SAFETY: Only one mutable reference is ever returned at a time,
            // and we consume the iterator.
            let map = &mut *self.map;
            map.get_mut(id)
        }
    }
}

// pub struct TracksIterMut<'a> {
//     map: &'a mut HashMap<TrackId, model::track::Track>,
//     order: &'a [u64], // Borrow `track_order` directly
//     index: usize,     // Track the current position manually
// }

// impl<'a> Iterator for TracksIterMut<'a> {
//     type Item = &'a mut model::track::Track;

//     fn next(&mut self) -> Option<Self::Item> {
//         if self.index >= self.order.len() {
//             return None;
//         }
//         let id = self.order[self.index]; // Get the track ID
//         self.index += 1; // Move to the next track

//         // SAFETY: We use split_at_mut to create two separate slices,
//         // ensuring no aliasing occurs while borrowing mutably.
//         unsafe {
//             let map = &mut *(self.map as *mut _);
//             map.get_mut(&id)
//         }
//     }
// }

// pub struct TracksIterMut<'a> {
//     map: &'a mut HashMap<TrackId, model::track::Track>,
//     order: std::vec::IntoIter<u64>, // Owning iterator to avoid borrow conflicts
// }

// impl<'a> Iterator for TracksIterMut<'a> {
//     type Item = &'a mut model::track::Track;

//     fn next(&mut self) -> Option<Self::Item> {
//         let id = self.order.next()?; // Get the next track ID
//         self.map.get_mut(&id) // Get a mutable reference to the track
//     }
// }
// pub struct TracksIterMut<'a> {
//     map: *mut HashMap<TrackId, model::track::Track>,
//     order: std::slice::Iter<'a, u64>,
// }

// impl<'a> Iterator for TracksIterMut<'a> {
//     type Item = &'a mut model::track::Track;

//     fn next(&mut self) -> Option<Self::Item> {
//         let id = self.order.next()?;
//         unsafe { (*self.map).get_mut(id) }
//     }
// }

// pub struct TracksIterMut<'a> {
//     map: &'a mut HashMap<TrackId, model::track::Track>,
//     order: std::slice::IterMut<'a, u64>,
// }

// impl<'a> Iterator for TracksIterMut<'a> {
//     type Item = &'a mut model::track::Track;
//     fn next(&mut self) -> Option<Self::Item> {
//         self.order.next().map(|id| self.map.get_mut(id).unwrap())
//     }
// }
