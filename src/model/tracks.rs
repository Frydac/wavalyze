use crate::{audio, model, pos};
use anyhow::Result;
use anyhow::anyhow;
use egui::ahash::HashMap;
use std::ops::Deref;

use crate::audio::SampleIx;
// use anyhow::Result;

pub type TrackId = crate::util::Id;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct TrackHoverInfo {
    pub track_id: TrackId,
    pub screen_pos: pos::Pos,
}

#[derive(Default, Debug, PartialEq, Clone)]
pub struct TracksHoverInfo {
    pub current: Option<TrackHoverInfo>,
}

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

    pub tracks_hover_info: TracksHoverInfo,
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

    pub fn get_total_buffer_range(&self) -> audio::SampleIxRange {
        let mut total_range = audio::SampleIxRange::new(0.0, 0.0);
        for track in self.iter() {
            total_range.include(track.buffer.borrow().nr_samples() as f64);
        }
        total_range
    }
}

impl Tracks {
    // pub fn set_screen_rect(&mut self, id: TrackId, screen_rect: rect::Rect) -> anyhow::Result<()> {
    //     let track = self.track_mut(id).ok_or(anyhow!(format!("Track with id {} not found", id)))?;

    //     // If the zoom level hasn't been set in a track, this call will trigger setting default
    //     // zoom: the track needs to screen width to calculate the view buffer
    //     track.set_screen_rect(screen_rect)?;

    //     // We expect this to be true?
    //     if let Some(spp) = track.samples_per_pixel() {
    //         // If this is the first time set_view_rect is called for any track, we get the
    //         // 'default' zoom level (zoom level that shows whole buffer on given screen width),
    //         // from that track and store it
    //         if self.samples_per_pixel.is_none() {
    //             self.samples_per_pixel = Some(spp);

    //             // Initialize all other tracks with the same zoom level
    //             for track in self.tracks.iter_mut() {
    //                 track.1.set_samples_per_pixel(spp);
    //             }
    //         }
    //     }
    //     Ok(())
    // }

    pub fn set_sample_rect(
        &mut self,
        id: TrackId,
        sample_rect: audio::SampleRect,
    ) -> anyhow::Result<()> {
        let track = self
            .track_mut(id)
            .ok_or(anyhow!(format!("Track with id {} not found", id)))?;

        // if let Some(track) = self.track_mut(id) {

        // }
        Ok(())
    }

    pub fn clear_tracks(&mut self) {
        self.tracks.clear();
        self.track_order.clear();
    }
}

impl Tracks {
    pub fn shift_x(&mut self, pixel_delta: f32) -> Result<()> {
        for track in self.tracks.values_mut() {
            track.shift_sample_rect_x(pixel_delta)?;
        }
        Ok(())
    }

    pub fn shift_y(&mut self, pixel_delta: f32) -> Result<()> {
        todo!()
    }

    /// * `zoom_center_x` - The x position of the center of the zoom
    /// * `zoom_delta` - The amount to zoom in or out
    pub fn zoom_x(&mut self, zoom_center_x: f32, zoom_delta: f32) -> Result<()> {
        for track in self.tracks.values_mut() {
            track.zoom_x(zoom_center_x, zoom_delta)?;
        }
        Ok(())
    }

    pub fn zoom_y(&mut self, zoom_center_y: f32, zoom_delta: f32, track_id: TrackId) -> Result<()> {
        todo!()
    }
}

// impl Tracks {
// TOOD: lol, doesn't belong here
// pub fn ui(&mut self, ui: &mut egui::Ui, model: &mut model::Model) {
//     // do hover interaction (and possibly other) on all tracks, so if no track is hovered, we
//     // can unhover all tracks after the loop
//     // depending on the interaction, update all the model tracks, and update the view tracks
//     //
//     // But that doens't work due to immediate mode, must draw when doing interaction
//     // So, the tracks above where the mouse is will not have the current mouse position, but
//     // the last one. We'll see if this is a problem.
//     let height_track = ui.available_height() / self.tracks.len() as f32;
//     let width_track = ui.available_width();
//     // for i in 0..self.tracks.len() {
//     //     let track_id = self.track_order[i];
//     //     ui.allocate_ui([width_track, height_track].into(), |ui| {
//     //         let view_track = &mut self.tracks.get_mut(&track_id).unwrap();
//     //         view_track.ui(ui, model);
//     //     });
//     // }

//     // set current_hover_info to None
//     // user previous_hover_info for each track
//     // call ui for each track
//     //    each track reports it's hover info
//     //      if one is hovered the current_hover_info reflects that
//     //          immediately update previous_hover_info?
//     //      if no track is hovered, current_hover_info is None at the end of the loop
//     //  after loop, previous_hover_info is set to current_hover_info

//     self.tracks_hover_info.previous = self.tracks_hover_info.current;
//     self.tracks_hover_info.current = None;

//     for i in 0..self.tracks.len() {
//         let track_id = self.track_order[i];
//         let track = self.tracks.get_mut(&track_id).unwrap();
//         if let Some(info) = self.tracks_hover_info.previous {
//             track.set_hover_info(info.screen_pos);
//         }

//         ui.allocate_ui([width_track, height_track].into(), |ui| {
//             let view_track = &mut self.tracks.get_mut(&track_id).unwrap();
//             // view_track.ui(ui, model);
//         });

//         if let Some(info) = self.tracks_hover_info.current {
//             self.tracks_hover_info.previous = Some(info);
//         }
//         //     let test = self.tracks_hover_info.clone().previous.clone();
//         // if let Some(info) = self.tracks_hover_info.previous.clone() {
//         //     track.set_hover_info(info.screen_pos);
//         // }
//         // track.ui(ui, model);
//     }
// }
// }
impl Tracks {
    // probably want to add some kind of y coordinate for selecting a rectangle
    pub fn select_start(&mut self, id: TrackId, sample: SampleIx) -> anyhow::Result<()> {
        todo!()
    }
    pub fn select_end(&mut self, id: TrackId, sample: SampleIx) -> anyhow::Result<()> {
        todo!()
    }

    // hover over a sample, state is retained until unhover is called
    pub fn update_hover_info(&mut self, id: TrackId, screen_pos: pos::Pos) {
        self.tracks_hover_info.current = Some(TrackHoverInfo {
            track_id: id,
            screen_pos,
        });
        for track in self.tracks.values_mut() {
            track.update_hover_info(screen_pos);
        }
    }
    pub fn unhover(&mut self) {
        for track in self.tracks.values_mut() {
            track.unhover();
        }
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
