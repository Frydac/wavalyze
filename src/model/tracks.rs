use crate::{
    audio::{self, BufferId},
    model::{
        PixelCoord, TimeCamera, action::SelectionEdge, config::TrackConfig, hover_info::HoverInfoE,
        ruler, selection_info::SelectionInfoE, time_camera, track,
    },
};
use anyhow::Result;
use slotmap::SlotMap;

use crate::{
    model::track::{Track, TrackId},
    wav::file::File,
};

#[derive(Debug, Clone)]
pub struct Tracks {
    pub ruler: ruler::Time,
    /// Shared X-axis camera (seconds → pixels). Drives both the ruler ticks and each
    /// track's visible sample window. Pan/zoom mutate this; the ruler only reads.
    pub time_camera: TimeCamera,
    pub tracks: SlotMap<TrackId, Track>,
    pub tracks_order: Vec<TrackId>,
    // hover
    pub hover_info: HoverInfoE,
    // selection
    pub selection_info: SelectionInfoE,
    // zoom
    pub available_height: f32,
    /// When enabled, visible tracks continuously share `available_height` equally instead of
    /// retaining their individually stored heights. This is runtime layout state and is not part
    /// of the persisted user configuration.
    pub equal_height_layout: bool,
    pub width_info: f32,
}

impl Default for Tracks {
    fn default() -> Self {
        Self {
            ruler: ruler::Time::default(),
            time_camera: TimeCamera::default(),
            tracks: SlotMap::default(),
            tracks_order: Vec::new(),
            hover_info: HoverInfoE::default(),
            selection_info: SelectionInfoE::default(),
            available_height: 0.0,
            equal_height_layout: true,
            width_info: 0.0,
        }
    }
}

impl Tracks {
    const SELECTION_EDGE_ZOOM_SAMPLES_PER_PIXEL: f64 = 0.1;
    const AUTO_FIT_PEAK_SCREEN_FRACTION: f64 = 0.95;

    pub fn visible_tracks_len(&self) -> usize {
        self.tracks.values().filter(|track| track.visible).count()
    }

    pub fn add_track_to_end(
        &mut self,
        buffer_id: BufferId,
        sample_rate: u32,
        track_config: &TrackConfig,
    ) -> Result<TrackId> {
        self.insert_track(
            buffer_id,
            sample_rate,
            self.tracks_order.len(),
            track_config,
        )
    }

    pub fn insert_track(
        &mut self,
        buffer_id: BufferId,
        sample_rate: u32,
        insert_ix: usize,
        track_config: &TrackConfig,
    ) -> Result<TrackId> {
        anyhow::ensure!(
            self.find_track(buffer_id).is_none(),
            "Track for buffer {:?} already exists",
            buffer_id
        );
        let track = Track::new2(buffer_id, sample_rate, track_config)?;
        let track_id = self.tracks.insert(track);
        let insert_ix = insert_ix.min(self.tracks_order.len());
        self.tracks_order.insert(insert_ix, track_id);
        Ok(track_id)
    }

    pub fn add_diff_track_to_end(
        &mut self,
        diff: track::diff::Diff,
        sample_rate: u32,
        track_config: &TrackConfig,
    ) -> Result<TrackId> {
        self.insert_diff_track(diff, sample_rate, self.tracks_order.len(), track_config)
    }

    /// Insert a diff track at `insert_ix` in the render order (clamped to the end).
    pub fn insert_diff_track(
        &mut self,
        diff: track::diff::Diff,
        sample_rate: u32,
        insert_ix: usize,
        track_config: &TrackConfig,
    ) -> Result<TrackId> {
        let track = Track::new_diff(diff, sample_rate, track_config)?;
        let track_id = self.tracks.insert(track);
        let insert_ix = insert_ix.min(self.tracks_order.len());
        self.tracks_order.insert(insert_ix, track_id);
        Ok(track_id)
    }

    pub fn remove_track(&mut self, track_id: TrackId) {
        self.tracks.remove(track_id);
        self.tracks_order.retain(|id| *id != track_id);
    }

    /// Move `track_id` to `to_gap_ix`, a gap index (`0..=len`) in the *current* `tracks_order`.
    /// No-op if the track is not present. Adjacent-gap moves resolve to no change.
    pub fn move_track(&mut self, track_id: TrackId, to_gap_ix: usize) {
        let Some(from) = self.tracks_order.iter().position(|id| *id == track_id) else {
            return;
        };
        let id = self.tracks_order.remove(from);
        // `to_gap_ix` was computed before the removal; gaps after `from` shift down by one.
        let to = if to_gap_ix > from {
            to_gap_ix - 1
        } else {
            to_gap_ix
        };
        self.tracks_order
            .insert(to.min(self.tracks_order.len()), id);
    }

    pub fn add_tracks_from_file(&mut self, file: &File, track_config: &TrackConfig) -> Result<()> {
        for (_ch_ix, channel) in file.channels.iter() {
            let track_id =
                self.add_track_to_end(channel.buffer_id, file.sample_rate, track_config)?;
            if let Some(track) = self.tracks.get_mut(track_id) {
                track.single.sample_ix_offset = file.sample_ix_offset as f64;
            }
        }
        Ok(())
    }

    pub fn remove_all_tracks(&mut self) {
        self.tracks.clear();
        self.tracks_order.clear();
    }

    pub fn find_track(&self, buffer_id: BufferId) -> Option<(TrackId, &Track)> {
        self.tracks
            .iter()
            .find(|(_, track)| track.single.buffer_id == buffer_id)
    }

    pub fn get_track(&self, track_id: TrackId) -> Option<&Track> {
        self.tracks.get(track_id)
    }

    pub fn get_track_mut(&mut self, track_id: TrackId) -> Option<&mut Track> {
        self.tracks.get_mut(track_id)
    }

    pub fn get_track_height(&self, track_id: TrackId) -> Option<f32> {
        self.tracks.get(track_id).map(|track| track.height)
    }

    pub fn set_track_visibility(&mut self, track_id: TrackId, visible: bool) {
        if let Some(track) = self.tracks.get_mut(track_id) {
            track.visible = visible;
        } else {
            tracing::warn!("Track {:?} not found", track_id);
        }
    }

    pub fn set_track_height(&mut self, track_id: TrackId, height: f32) {
        if let Some(track) = self.tracks.get_mut(track_id) {
            track.height = height;
        } else {
            tracing::warn!("Track {:?} not found", track_id);
        }
    }

    pub fn set_tracks_height(&mut self, height: f32) {
        for track in self.tracks.values_mut() {
            track.height = height;
        }
    }

    // Updates sample_rect.val_rng
    pub fn pan_track_value_range(
        &mut self,
        track_id: TrackId,
        delta_pixels: f32,
        display_scale: ruler::ValueDisplayScale,
    ) -> Result<()> {
        let track = self
            .tracks
            .get_mut(track_id)
            .ok_or_else(|| anyhow::anyhow!("Track {:?} not found", track_id))?;
        let screen_rect = track
            .screen_rect
            .ok_or_else(|| anyhow::anyhow!("screen_rect is missing"))?;
        let sample_rect = track
            .single
            .sample_rect_raw()
            .ok_or_else(|| anyhow::anyhow!("sample_rect is missing"))?;
        let Some(val_rng) = sample_rect.val_rng() else {
            return Ok(());
        };

        let shifted = ruler::value::pan_val_range_with_scale(
            val_rng,
            delta_pixels,
            screen_rect,
            display_scale,
        );
        let mut sample_rect = sample_rect;
        sample_rect.set_val_rng(shifted);
        track.single.set_sample_rect(sample_rect);
        Ok(())
    }

    /// Resolve the buffer-local, half-open sample range used by a per-track Y auto-fit request.
    /// A valid global selection takes precedence; otherwise the track's visible sample range is
    /// used. Selection indices use the reference track's sample rate, so they are projected through
    /// time before being mapped to the target track.
    pub fn auto_fit_local_ix_range(
        &self,
        track_id: TrackId,
        audio: &audio::manager::AudioManager,
    ) -> Result<Option<audio::sample::IxRange>> {
        let track = self
            .tracks
            .get(track_id)
            .ok_or_else(|| anyhow::anyhow!("Track {:?} not found", track_id))?;
        let buffer_len = i64::try_from(audio.get_buffer(track.single.buffer_id)?.nr_samples())
            .map_err(|_| anyhow::anyhow!("Buffer is too large to address with sample indices"))?;

        let global_range = match self.selection_info {
            SelectionInfoE::IsSelected(selection)
                if selection.ix_rng.end > selection.ix_rng.start =>
            {
                let reference_rate = self
                    .reference_sample_rate()
                    .ok_or_else(|| anyhow::anyhow!("No reference sample rate"))?;
                let start_time =
                    time_camera::sample_ix_to_time(selection.ix_rng.start as f64, reference_rate);
                let end_time =
                    time_camera::sample_ix_to_time(selection.ix_rng.end as f64, reference_rate);
                audio::sample::FracIxRange {
                    start: time_camera::time_to_sample_ix(start_time, track.sample_rate),
                    end: time_camera::time_to_sample_ix(end_time, track.sample_rate),
                }
            }
            _ => {
                let sample_rect = track
                    .single
                    .sample_rect_raw()
                    .ok_or_else(|| anyhow::anyhow!("sample_rect is missing"))?;
                sample_rect.ix_rng()
            }
        };

        let local_start = global_range.start + track.single.sample_ix_offset;
        let local_end = global_range.end + track.single.sample_ix_offset;
        if !local_start.is_finite() || !local_end.is_finite() || local_end <= local_start {
            return Ok(None);
        }

        // Integer sample n is in the fractional half-open interval [start, end) when
        // ceil(start) <= n < ceil(end). Clamp after applying the track offset so jobs can index
        // directly into the buffer.
        let start = (local_start.ceil() as i64).clamp(0, buffer_len);
        let end = (local_end.ceil() as i64).clamp(0, buffer_len);
        if end <= start {
            return Ok(None);
        }

        Ok(Some(audio::sample::IxRange { start, end }))
    }

    /// Apply a detected peak to a track's value camera while keeping sample zero centered.
    ///
    /// Returns `false` without changing the track when the asynchronous result is stale, the peak
    /// is absent/invalid/silent, or the track has no initialized sample rectangle.
    pub fn apply_auto_fit_peak(
        &mut self,
        track_id: TrackId,
        expected_buffer_id: BufferId,
        peak_norm: Option<f64>,
        display_scale: ruler::ValueDisplayScale,
    ) -> bool {
        let Some(track) = self.tracks.get_mut(track_id) else {
            return false;
        };
        if track.single.buffer_id != expected_buffer_id {
            return false;
        }
        let Some(peak) = peak_norm
            .map(f64::abs)
            .filter(|peak| peak.is_finite() && *peak > 0.0)
        else {
            return false;
        };
        let Some(mut sample_rect) = track.single.sample_rect_raw() else {
            return false;
        };

        // ValueDisplayScale transforms each unit interval independently. Convert the magnitude to
        // the corresponding piecewise display coordinate, then leave 5% of each canvas half free.
        let whole = peak.floor();
        let display_peak = whole + display_scale.sample_to_display(peak - whole);
        let extent = display_peak / Self::AUTO_FIT_PEAK_SCREEN_FRACTION;
        if !extent.is_finite() || extent <= 0.0 {
            return false;
        }

        sample_rect.set_val_rng(audio::sample::ValRange {
            min: -extent,
            max: extent,
        });
        track.single.set_sample_rect(sample_rect);
        true
    }

    /// Reset the value range to full scale (pan/zoom reset) for a single track.
    pub fn recenter_track_value_range(&mut self, track_id: TrackId) -> Result<()> {
        let track = self
            .tracks
            .get_mut(track_id)
            .ok_or_else(|| anyhow::anyhow!("Track {:?} not found", track_id))?;
        let mut sample_rect = track
            .single
            .sample_rect_raw()
            .ok_or_else(|| anyhow::anyhow!("sample_rect is missing"))?;
        let Some(val_rng) = sample_rect.val_rng() else {
            return Ok(());
        };

        sample_rect.set_val_rng(audio::sample::ValRange {
            min: -1.0,
            max: 1.0,
        });
        track.single.set_sample_rect(sample_rect);
        Ok(())
    }

    /// Reset the value range to full scale (pan/zoom reset) for all tracks.
    pub fn recenter_all_value_ranges(&mut self) -> Result<()> {
        for track_id in self.tracks_order.clone() {
            if self.tracks.get(track_id).is_some_and(|track| track.visible) {
                self.recenter_track_value_range(track_id)?;
            }
        }
        Ok(())
    }

    pub fn zoom_track_value_range(
        &mut self,
        track_id: TrackId,
        delta_pixels: f32,
        center_y: f32,
        display_scale: ruler::ValueDisplayScale,
    ) -> Result<()> {
        if delta_pixels == 0.0 {
            return Ok(());
        }
        let track = self
            .tracks
            .get_mut(track_id)
            .ok_or_else(|| anyhow::anyhow!("Track {:?} not found", track_id))?;
        let screen_rect = track
            .screen_rect
            .ok_or_else(|| anyhow::anyhow!("screen_rect is missing"))?;
        let mut sample_rect = track
            .single
            .sample_rect_raw()
            .ok_or_else(|| anyhow::anyhow!("sample_rect is missing"))?;
        let Some(val_rng) = sample_rect.val_rng() else {
            return Ok(());
        };
        let zoomed = ruler::value::zoom_val_range_with_scale(
            val_rng,
            delta_pixels,
            center_y,
            screen_rect,
            display_scale,
        );
        if zoomed.is_empty() {
            return Ok(());
        }
        sample_rect.set_val_rng(zoomed);
        track.single.set_sample_rect(sample_rect);
        Ok(())
    }

    /// Push the camera's current time window down into each track's sample_rect, converting
    /// seconds → sample-ix per-track using each buffer's own sample_rate. Call this whenever
    /// the camera changes (pan, zoom, zoom-to-*).
    pub fn update_tracks_to_camera(&mut self, audio: &audio::manager::AudioManager) -> Result<()> {
        let screen_width = self.ruler.screen_rect().width() as f64;
        anyhow::ensure!(screen_width > 0.0, "Ruler screen rect width is zero");
        let time_range = self.time_camera.time_range(screen_width);

        for track in self.tracks.values_mut() {
            let ix_range = audio::sample::FracIxRange {
                start: time_camera::time_to_sample_ix(time_range.start, track.sample_rate),
                end: time_camera::time_to_sample_ix(time_range.end, track.sample_rate),
            };
            track.single.set_ix_range(ix_range, audio)?;
        }

        Ok(())
    }

    /// Reference sample_rate for converting between sample-ix and seconds at the *whole-view*
    /// level (ruler ticks, global selection ix). Returns the longest visible track's rate, or
    /// any visible track's rate as a fallback.
    /// TODO: once multi-rate display is real, the ruler should pick its display unit (seconds)
    /// independently rather than projecting through a single reference rate.
    pub fn reference_sample_rate(&self) -> Option<u32> {
        let mut best: Option<(u32, u64)> = None;
        for track in self.tracks.values() {
            if !track.visible {
                continue;
            }
            let width = track
                .single
                .sample_rect_raw()
                .map(|r| r.width() as u64)
                .unwrap_or_default();
            if best.is_none_or(|(_, w)| w < width) {
                best = Some((track.sample_rate, width));
            }
        }
        best.map(|(rate, _)| rate)
            .or_else(|| self.tracks.values().next().map(|t| t.sample_rate))
    }

    /// Visible sample-ix range projected through [`Self::reference_sample_rate`] — for ruler
    /// ticks and other "global" sample-ix display surfaces.
    pub fn ix_range(&self) -> Option<audio::sample::FracIxRange> {
        let sample_rate = self.reference_sample_rate()?;
        let screen_width = self.ruler.screen_rect().width() as f64;
        if screen_width <= 0.0 {
            return None;
        }
        let time_range = self.time_camera.time_range(screen_width);
        Some(audio::sample::FracIxRange {
            start: time_camera::time_to_sample_ix(time_range.start, sample_rate),
            end: time_camera::time_to_sample_ix(time_range.end, sample_rate),
        })
    }

    fn visible_tracks_time_bounds(
        &self,
        audio: &audio::manager::AudioManager,
    ) -> Option<std::ops::Range<f64>> {
        let mut bounds: Option<std::ops::Range<f64>> = None;
        for track in self.tracks.values() {
            if !track.visible {
                continue;
            }
            let buffer_id = track.single.buffer_id;
            let buffer = audio.get_buffer(buffer_id).ok()?;
            let nr_samples = buffer.nr_samples() as u64;
            let start_sample_ix = -track.single.sample_ix_offset;
            let end_sample_ix = nr_samples as f64 - track.single.sample_ix_offset;
            let start_s = time_camera::sample_ix_to_time(start_sample_ix, track.sample_rate);
            let end_s = time_camera::sample_ix_to_time(end_sample_ix, track.sample_rate);
            match bounds {
                Some(ref mut bounds) => {
                    bounds.start = bounds.start.min(start_s);
                    bounds.end = bounds.end.max(end_s);
                }
                None => {
                    bounds = Some(start_s..end_s);
                }
            }
        }
        bounds
    }

    /// Zoom to fit the longest visible track (in *seconds*, sample-rate-aware).
    pub fn zoom_to_full(&mut self, audio: &audio::manager::AudioManager) -> Result<()> {
        anyhow::ensure!(
            self.ruler.screen_rect().width() > 0.0,
            "Ruler screen rect width is zero"
        );
        let time_bounds = self
            .visible_tracks_time_bounds(audio)
            .ok_or_else(|| anyhow::anyhow!("No tracks"))?;
        let duration_s = time_bounds.end - time_bounds.start;
        let screen_width = self.ruler.screen_rect().width() as f64;
        self.time_camera.time_start = time_bounds.start;
        self.time_camera
            .set_seconds_per_pixel(duration_s / screen_width);
        self.update_tracks_to_camera(audio)?;
        Ok(())
    }

    pub fn zoom_to_selection(&mut self, audio: &audio::manager::AudioManager) -> Result<()> {
        let SelectionInfoE::IsSelected(selection_info) = self.selection_info else {
            return Ok(());
        };
        if selection_info.ix_rng.end <= selection_info.ix_rng.start {
            return Ok(());
        }
        anyhow::ensure!(
            self.ruler.screen_rect().width() > 0.0,
            "Ruler screen rect width is zero"
        );
        let sample_rate = self
            .reference_sample_rate()
            .ok_or_else(|| anyhow::anyhow!("No tracks"))?;
        self.zoom_to_time_range_clamped(std::ops::Range {
            start: time_camera::sample_ix_to_time(selection_info.ix_rng.start as f64, sample_rate),
            end: time_camera::sample_ix_to_time(selection_info.ix_rng.end as f64, sample_rate),
        });
        self.update_tracks_to_camera(audio)?;
        Ok(())
    }

    pub fn zoom_to_selection_edge(
        &mut self,
        audio: &audio::manager::AudioManager,
        edge: SelectionEdge,
    ) -> Result<()> {
        let SelectionInfoE::IsSelected(selection_info) = self.selection_info else {
            return Ok(());
        };
        if selection_info.ix_rng.end <= selection_info.ix_rng.start {
            return Ok(());
        }
        anyhow::ensure!(
            self.ruler.screen_rect().width() > 0.0,
            "Ruler screen rect width is zero"
        );
        let sample_rate = self
            .reference_sample_rate()
            .ok_or_else(|| anyhow::anyhow!("No tracks"))?;

        let edge_ix = match edge {
            SelectionEdge::Left => selection_info.ix_rng.start as f64,
            SelectionEdge::Right => selection_info.ix_rng.end as f64,
        };
        let edge_time = time_camera::sample_ix_to_time(edge_ix, sample_rate);
        let screen_width = self.ruler.screen_rect().width() as f64;
        let visible_len_samples = screen_width * Self::SELECTION_EDGE_ZOOM_SAMPLES_PER_PIXEL;
        let visible_len_s = time_camera::sample_ix_to_time(visible_len_samples, sample_rate);
        let half = visible_len_s / 2.0;
        self.time_camera
            .set_seconds_per_pixel(visible_len_s / screen_width);
        self.time_camera.time_start = edge_time - half;
        self.update_tracks_to_camera(audio)?;
        Ok(())
    }

    /// Update track heights to equally distribute the available height, taking min_height into account.
    pub fn fill_screen_height(&mut self, min_height: f32) -> Result<()> {
        let visible_tracks = self.visible_tracks_len();
        if visible_tracks == 0 {
            return Ok(());
        }
        let track_height = self.available_height / visible_tracks as f32;
        let min_total_height = min_height + track::HEADER_HEIGHT;
        for track in self.tracks.values_mut() {
            if track.visible {
                track.height = track_height.max(min_total_height);
            }
        }
        Ok(())
    }

    /// `samples_per_pixel` at the reference sample rate. None until a track is present so
    /// callers can no-op cleanly during the empty-model window after startup.
    pub fn samples_per_pixel(&self) -> Option<f64> {
        let sample_rate = self.reference_sample_rate()?;
        Some(self.time_camera.seconds_per_pixel() * sample_rate as f64)
    }
}

impl Tracks {
    /// Global sample-ix → screen-x using the reference sample rate. None when no tracks
    /// exist or the screen rect hasn't been claimed yet. Uses the stable bin-based mapping
    /// from `ruler::util::sample_ix_to_screen_x` so adjacent sample-level renders match what
    /// the ruler ticks do.
    pub fn sample_ix_to_screen_x(&self, sample_ix: f64) -> Option<f32> {
        let ix_range = self.ix_range()?;
        Some(crate::model::ruler::sample_ix_to_screen_x(
            sample_ix,
            ix_range,
            *self.ruler.screen_rect(),
        ))
    }

    pub fn screen_x_to_sample_ix(&self, screen_x: f32) -> Option<f64> {
        let ix_range = self.ix_range()?;
        Some(crate::model::ruler::screen_x_to_sample_ix(
            screen_x,
            ix_range,
            *self.ruler.screen_rect(),
        ))
    }
}

// X-axis camera mutation: pan / zoom. Mirror of what `ruler::Time` used to do, now anchored
// on the `time_camera` field. Callers must follow up with `update_tracks_to_camera` so each
// track's sample-ix window stays in sync with the camera.
const MIN_SECONDS_PER_PIXEL_MULT: f64 = 0.002;

impl Tracks {
    pub fn pan_x(&mut self, delta_pixels: PixelCoord) {
        let delta_s = delta_pixels as f64 * self.time_camera.seconds_per_pixel();
        self.time_camera.time_start += delta_s;
        // Keep the cursor-anchored sample-ix in sync after panning so the displayed value
        // stays under the cursor.
        let screen_x = match self.hover_info {
            crate::model::hover_info::HoverInfoE::IsHovered(ref hi) => Some(hi.screen_pos.x),
            crate::model::hover_info::HoverInfoE::NotHovered => None,
        };
        if let Some(screen_x) = screen_x
            && let Some(sample_ix) = self.sample_ix_for_screen_x_unchecked(screen_x)
            && let crate::model::hover_info::HoverInfoE::IsHovered(ref mut hi) = self.hover_info
        {
            hi.sample_ix = sample_ix;
        }
    }

    pub fn zoom_x(&mut self, nr_pixels: f32, center_x: f32) {
        let screen_rect = *self.ruler.screen_rect();
        if !screen_rect.contains_x(center_x) || screen_rect.width() <= 0.0 {
            return;
        }
        let Some(sample_rate) = self.reference_sample_rate() else {
            return;
        };

        let center_x_norm = center_x - screen_rect.left();
        let frac_min = center_x_norm / screen_rect.width();
        let new_min_x = screen_rect.left() - frac_min * nr_pixels;
        let new_max_x = screen_rect.right() + (1.0 - frac_min) * nr_pixels;

        let new_min_t = self.time_camera.screen_x_to_time(new_min_x, screen_rect);
        let new_max_t = self.time_camera.screen_x_to_time(new_max_x, screen_rect);

        let new_seconds_per_pixel = (new_max_t - new_min_t) / screen_rect.width() as f64;
        let min_spp = MIN_SECONDS_PER_PIXEL_MULT / sample_rate as f64;
        if new_seconds_per_pixel > min_spp {
            self.time_camera.time_start = new_min_t;
            self.time_camera
                .set_seconds_per_pixel(new_seconds_per_pixel);
        }
    }

    pub fn zoom_to_time_range(&mut self, time_range: std::ops::Range<f64>) {
        let screen_width = self.ruler.screen_rect().width() as f64;
        if screen_width <= 0.0 {
            return;
        }
        self.time_camera
            .set_seconds_per_pixel((time_range.end - time_range.start) / screen_width);
        self.time_camera.time_start = time_range.start;
    }

    pub fn zoom_to_time_range_clamped(&mut self, time_range: std::ops::Range<f64>) {
        let screen_width = self.ruler.screen_rect().width() as f64;
        if screen_width <= 0.0 {
            return;
        }
        let Some(sample_rate) = self.reference_sample_rate() else {
            self.zoom_to_time_range(time_range);
            return;
        };
        let requested_spp = (time_range.end - time_range.start) / screen_width;
        let min_spp = MIN_SECONDS_PER_PIXEL_MULT / sample_rate as f64;
        let seconds_per_pixel = requested_spp.max(min_spp);
        self.time_camera.set_seconds_per_pixel(seconds_per_pixel);

        let time_start = if seconds_per_pixel > requested_spp {
            let visible_len_s = screen_width * seconds_per_pixel;
            (time_range.start + time_range.end) / 2.0 - visible_len_s / 2.0
        } else {
            time_range.start
        };
        self.time_camera.time_start = time_start;
    }

    /// `screen_x → sample_ix` without bounds checks. Used to refresh a stored hover sample_ix
    /// after panning so the displayed value stays under the cursor.
    fn sample_ix_for_screen_x_unchecked(&self, screen_x: f32) -> Option<f64> {
        let sample_rate = self.reference_sample_rate()?;
        let time = self
            .time_camera
            .screen_x_to_time(screen_x, *self.ruler.screen_rect());
        Some(time_camera::time_to_sample_ix(time, sample_rate))
    }
}

#[cfg(test)]
mod tests {
    use super::Tracks;
    use crate::{
        audio,
        model::selection_info::{SelectionInfo, SelectionInfoE},
        model::{action::SelectionEdge, config::TrackConfig, ruler::ValueDisplayScale},
        rect::Rect,
    };

    const TEST_SAMPLE_RATE: u32 = 48_000;

    fn insert_buffer(
        audio: &mut audio::manager::AudioManager,
        nr_samples: usize,
    ) -> audio::BufferId {
        insert_buffer_at_rate(audio, nr_samples, TEST_SAMPLE_RATE)
    }

    fn insert_buffer_at_rate(
        audio: &mut audio::manager::AudioManager,
        nr_samples: usize,
        sample_rate: u32,
    ) -> audio::BufferId {
        let buffer = audio::buffer::BufferE::F32(audio::buffer::Buffer::with_size(
            sample_rate,
            32,
            nr_samples,
        ));
        audio.buffers.insert(std::sync::Arc::new(buffer))
    }

    fn track_with_visible_range(
        tracks: &mut Tracks,
        audio: &mut audio::manager::AudioManager,
        sample_rate: u32,
        nr_samples: usize,
        visible_range: std::ops::Range<f64>,
    ) -> crate::model::track::TrackId {
        let buffer_id = insert_buffer_at_rate(audio, nr_samples, sample_rate);
        let track_id = tracks
            .add_track_to_end(buffer_id, sample_rate, &TrackConfig::default())
            .unwrap();
        let mut sample_rect = audio::SampleRect::from_buffere(audio.get_buffer(buffer_id).unwrap());
        sample_rect.set_ix_rng(visible_range.into());
        tracks
            .get_track_mut(track_id)
            .unwrap()
            .single
            .set_sample_rect(sample_rect);
        track_id
    }

    /// Most tests need at least one track so `reference_sample_rate()` resolves and
    /// `zoom_to_*` / `samples_per_pixel()` produce results.
    fn seed_track(
        tracks: &mut Tracks,
        audio: &mut audio::manager::AudioManager,
        nr_samples: usize,
    ) {
        let buffer_id = insert_buffer(audio, nr_samples);
        tracks
            .add_track_to_end(buffer_id, TEST_SAMPLE_RATE, &TrackConfig::default())
            .unwrap();
    }

    fn track_with_value_range(
        tracks: &mut Tracks,
        audio: &mut audio::manager::AudioManager,
        val_rng: audio::sample::ValRange<f64>,
    ) -> crate::model::track::TrackId {
        let track_id = tracks
            .add_track_to_end(
                insert_buffer(audio, 64),
                TEST_SAMPLE_RATE,
                &TrackConfig::default(),
            )
            .unwrap();
        let track = tracks.get_track_mut(track_id).unwrap();
        track.set_screen_rect(Rect::new(0.0, 0.0, 100.0, 100.0));
        let mut sample_rect =
            audio::SampleRect::from_buffere(audio.get_buffer(track.single.buffer_id).unwrap());
        sample_rect.set_val_rng(val_rng);
        track.single.set_sample_rect(sample_rect);
        track_id
    }

    #[test]
    fn auto_fit_range_prefers_valid_selection_over_visible_range() {
        let mut tracks = Tracks::default();
        let mut audio = audio::manager::AudioManager::default();
        let track_id =
            track_with_visible_range(&mut tracks, &mut audio, TEST_SAMPLE_RATE, 100, 2.0..8.0);
        tracks.selection_info = SelectionInfoE::IsSelected(SelectionInfo {
            ix_rng: (20..30).into(),
            screen_x_start: 0.0,
            screen_x_end: 0.0,
        });

        assert_eq!(
            tracks.auto_fit_local_ix_range(track_id, &audio).unwrap(),
            Some((20..30).into())
        );
    }

    #[test]
    fn auto_fit_range_uses_fractional_visible_range_without_selection() {
        let mut tracks = Tracks::default();
        let mut audio = audio::manager::AudioManager::default();
        let track_id =
            track_with_visible_range(&mut tracks, &mut audio, TEST_SAMPLE_RATE, 100, 10.2..19.1);

        assert_eq!(
            tracks.auto_fit_local_ix_range(track_id, &audio).unwrap(),
            Some((11..20).into())
        );
    }

    #[test]
    fn auto_fit_range_converts_selection_to_target_sample_rate() {
        let mut tracks = Tracks::default();
        let mut audio = audio::manager::AudioManager::default();
        // The wider sample rect establishes 48 kHz as the reference selection rate.
        track_with_visible_range(&mut tracks, &mut audio, 48_000, 2_000, 0.0..1_000.0);
        let target = track_with_visible_range(&mut tracks, &mut audio, 96_000, 1_000, 0.0..100.0);
        tracks.selection_info = SelectionInfoE::IsSelected(SelectionInfo {
            ix_rng: (10..20).into(),
            screen_x_start: 0.0,
            screen_x_end: 0.0,
        });

        assert_eq!(
            tracks.auto_fit_local_ix_range(target, &audio).unwrap(),
            Some((20..40).into())
        );
    }

    #[test]
    fn auto_fit_range_applies_track_offset() {
        let mut tracks = Tracks::default();
        let mut audio = audio::manager::AudioManager::default();
        let target =
            track_with_visible_range(&mut tracks, &mut audio, TEST_SAMPLE_RATE, 100, 0.0..10.0);
        tracks
            .get_track_mut(target)
            .unwrap()
            .single
            .sample_ix_offset = 5.0;
        tracks.selection_info = SelectionInfoE::IsSelected(SelectionInfo {
            ix_rng: (10..20).into(),
            screen_x_start: 0.0,
            screen_x_end: 0.0,
        });

        assert_eq!(
            tracks.auto_fit_local_ix_range(target, &audio).unwrap(),
            Some((15..25).into())
        );
    }

    #[test]
    fn auto_fit_range_clamps_partial_overlap_to_buffer() {
        let mut tracks = Tracks::default();
        let mut audio = audio::manager::AudioManager::default();
        let target =
            track_with_visible_range(&mut tracks, &mut audio, TEST_SAMPLE_RATE, 100, 0.0..10.0);
        tracks.selection_info = SelectionInfoE::IsSelected(SelectionInfo {
            ix_rng: (-5..5).into(),
            screen_x_start: 0.0,
            screen_x_end: 0.0,
        });

        assert_eq!(
            tracks.auto_fit_local_ix_range(target, &audio).unwrap(),
            Some((0..5).into())
        );
    }

    #[test]
    fn auto_fit_range_returns_none_when_chosen_range_misses_buffer() {
        let mut tracks = Tracks::default();
        let mut audio = audio::manager::AudioManager::default();
        let target =
            track_with_visible_range(&mut tracks, &mut audio, TEST_SAMPLE_RATE, 100, 0.0..10.0);
        tracks.selection_info = SelectionInfoE::IsSelected(SelectionInfo {
            ix_rng: (200..210).into(),
            screen_x_start: 0.0,
            screen_x_end: 0.0,
        });

        assert_eq!(
            tracks.auto_fit_local_ix_range(target, &audio).unwrap(),
            None
        );
    }

    #[test]
    fn auto_fit_range_falls_back_for_invalid_selection_and_rejects_empty_view() {
        let mut tracks = Tracks::default();
        let mut audio = audio::manager::AudioManager::default();
        let target =
            track_with_visible_range(&mut tracks, &mut audio, TEST_SAMPLE_RATE, 100, 7.0..9.0);
        tracks.selection_info = SelectionInfoE::IsSelected(SelectionInfo {
            ix_rng: (10..10).into(),
            screen_x_start: 0.0,
            screen_x_end: 0.0,
        });
        assert_eq!(
            tracks.auto_fit_local_ix_range(target, &audio).unwrap(),
            Some((7..9).into())
        );

        let mut sample_rect = tracks
            .get_track(target)
            .unwrap()
            .single
            .sample_rect_raw()
            .unwrap();
        sample_rect.set_ix_rng((12.0..12.0).into());
        tracks
            .get_track_mut(target)
            .unwrap()
            .single
            .set_sample_rect(sample_rect);
        assert_eq!(
            tracks.auto_fit_local_ix_range(target, &audio).unwrap(),
            None
        );
    }

    fn assert_peak_has_expected_headroom(
        tracks: &Tracks,
        track_id: crate::model::track::TrackId,
        peak: f64,
        display_scale: ValueDisplayScale,
    ) {
        let track = tracks.get_track(track_id).unwrap();
        let screen_rect = track.screen_rect.unwrap();
        let val_range = track.single.sample_rect_raw().unwrap().val_rng().unwrap();
        let peak_y = crate::model::ruler::sample_value_to_screen_y(
            peak,
            val_range,
            screen_rect,
            display_scale,
        )
        .unwrap();
        let used_half =
            (screen_rect.center().y - peak_y) as f64 / (screen_rect.height() as f64 / 2.0);

        assert!((used_half - 0.95).abs() < 1e-5);
    }

    #[test]
    fn auto_fit_peak_centers_zero_and_leaves_linear_headroom() {
        let mut tracks = Tracks::default();
        let mut audio = audio::manager::AudioManager::default();
        let track_id = track_with_value_range(
            &mut tracks,
            &mut audio,
            audio::sample::ValRange {
                min: -1.0,
                max: 1.0,
            },
        );
        let buffer_id = tracks.get_track(track_id).unwrap().single.buffer_id;

        assert!(tracks.apply_auto_fit_peak(
            track_id,
            buffer_id,
            Some(0.5),
            ValueDisplayScale::default(),
        ));

        let val_range = tracks
            .get_track(track_id)
            .unwrap()
            .single
            .sample_rect_raw()
            .unwrap()
            .val_rng()
            .unwrap();
        assert_eq!(val_range.min, -val_range.max);
        assert_peak_has_expected_headroom(&tracks, track_id, 0.5, ValueDisplayScale::default());
    }

    #[test]
    fn auto_fit_peak_respects_skewed_visual_headroom() {
        let mut tracks = Tracks::default();
        let mut audio = audio::manager::AudioManager::default();
        let track_id = track_with_value_range(
            &mut tracks,
            &mut audio,
            audio::sample::ValRange {
                min: -1.0,
                max: 1.0,
            },
        );
        let buffer_id = tracks.get_track(track_id).unwrap().single.buffer_id;
        let display_scale = ValueDisplayScale { skew_factor: 1.0 };

        assert!(tracks.apply_auto_fit_peak(track_id, buffer_id, Some(0.5), display_scale));
        assert_peak_has_expected_headroom(&tracks, track_id, 0.5, display_scale);
    }

    #[test]
    fn auto_fit_peak_treats_negative_peak_symmetrically() {
        let mut tracks = Tracks::default();
        let mut audio = audio::manager::AudioManager::default();
        let track_id = track_with_value_range(
            &mut tracks,
            &mut audio,
            audio::sample::ValRange {
                min: -1.0,
                max: 1.0,
            },
        );
        let buffer_id = tracks.get_track(track_id).unwrap().single.buffer_id;

        assert!(tracks.apply_auto_fit_peak(
            track_id,
            buffer_id,
            Some(-0.8),
            ValueDisplayScale::default(),
        ));
        let val_range = tracks
            .get_track(track_id)
            .unwrap()
            .single
            .sample_rect_raw()
            .unwrap()
            .val_rng()
            .unwrap();
        assert_eq!(val_range.min, -val_range.max);
        assert_peak_has_expected_headroom(&tracks, track_id, 0.8, ValueDisplayScale::default());
    }

    #[test]
    fn auto_fit_peak_ignores_stale_and_removed_tracks() {
        let mut tracks = Tracks::default();
        let mut audio = audio::manager::AudioManager::default();
        let track_id = track_with_value_range(
            &mut tracks,
            &mut audio,
            audio::sample::ValRange {
                min: -1.0,
                max: 1.0,
            },
        );
        let stale_buffer_id = insert_buffer(&mut audio, 64);
        let original_rect = tracks.get_track(track_id).unwrap().single.sample_rect_raw();

        assert!(!tracks.apply_auto_fit_peak(
            track_id,
            stale_buffer_id,
            Some(0.5),
            ValueDisplayScale::default(),
        ));
        assert_eq!(
            tracks.get_track(track_id).unwrap().single.sample_rect_raw(),
            original_rect
        );

        tracks.remove_track(track_id);
        assert!(!tracks.apply_auto_fit_peak(
            track_id,
            stale_buffer_id,
            Some(0.5),
            ValueDisplayScale::default(),
        ));
    }

    #[test]
    fn auto_fit_peak_ignores_silence_and_invalid_results() {
        let mut tracks = Tracks::default();
        let mut audio = audio::manager::AudioManager::default();
        let track_id = track_with_value_range(
            &mut tracks,
            &mut audio,
            audio::sample::ValRange {
                min: -1.0,
                max: 1.0,
            },
        );
        let buffer_id = tracks.get_track(track_id).unwrap().single.buffer_id;
        let original_rect = tracks.get_track(track_id).unwrap().single.sample_rect_raw();

        for peak in [None, Some(0.0), Some(f64::NAN), Some(f64::INFINITY)] {
            assert!(!tracks.apply_auto_fit_peak(
                track_id,
                buffer_id,
                peak,
                ValueDisplayScale::default(),
            ));
            assert_eq!(
                tracks.get_track(track_id).unwrap().single.sample_rect_raw(),
                original_rect
            );
        }
    }

    #[test]
    fn zoom_to_selection_fits_selected_range() {
        let mut tracks = Tracks::default();
        let mut audio = audio::manager::AudioManager::default();
        seed_track(&mut tracks, &mut audio, 1_000);
        tracks
            .ruler
            .set_screen_rect(Rect::new(0.0, 0.0, 1000.0, 100.0));
        tracks.selection_info = SelectionInfoE::IsSelected(SelectionInfo {
            ix_rng: (100..300).into(),
            screen_x_start: 10.0,
            screen_x_end: 30.0,
        });

        tracks.zoom_to_selection(&audio).unwrap();

        let ix_range = tracks.ix_range().unwrap();
        assert!((ix_range.start - 100.0).abs() < 1e-9);
        assert!((ix_range.end - 300.0).abs() < 1e-9);
        assert!((tracks.samples_per_pixel().unwrap() - 0.2).abs() < 1e-9);
    }

    #[test]
    fn zoom_to_selection_clamps_to_max_zoom_and_centers_selection() {
        let mut tracks = Tracks::default();
        let mut audio = audio::manager::AudioManager::default();
        seed_track(&mut tracks, &mut audio, 1_000);
        tracks
            .ruler
            .set_screen_rect(Rect::new(0.0, 0.0, 1000.0, 100.0));
        tracks.selection_info = SelectionInfoE::IsSelected(SelectionInfo {
            ix_rng: (100..101).into(),
            screen_x_start: 10.0,
            screen_x_end: 11.0,
        });

        tracks.zoom_to_selection(&audio).unwrap();

        let ix_range = tracks.ix_range().unwrap();
        let selection_center = 100.5;
        let view_center = (ix_range.start + ix_range.end) / 2.0;
        assert_eq!(tracks.samples_per_pixel(), Some(0.002));
        assert!((view_center - selection_center).abs() < 1e-9);
    }

    #[test]
    fn zoom_to_selection_without_selection_is_noop() {
        let mut tracks = Tracks::default();
        tracks
            .ruler
            .set_screen_rect(Rect::new(0.0, 0.0, 1000.0, 100.0));

        tracks
            .zoom_to_selection(&audio::manager::AudioManager::default())
            .unwrap();

        assert_eq!(tracks.ix_range(), None);
    }

    #[test]
    fn zoom_to_selection_with_invalid_range_is_noop() {
        let mut tracks = Tracks::default();
        tracks
            .ruler
            .set_screen_rect(Rect::new(0.0, 0.0, 1000.0, 100.0));
        tracks.selection_info = SelectionInfoE::IsSelected(SelectionInfo {
            ix_rng: (100..100).into(),
            screen_x_start: 10.0,
            screen_x_end: 10.0,
        });

        tracks
            .zoom_to_selection(&audio::manager::AudioManager::default())
            .unwrap();

        assert_eq!(tracks.ix_range(), None);
    }

    #[test]
    fn zoom_to_selection_left_edge_centers_edge_and_uses_sample_level_zoom() {
        let mut tracks = Tracks::default();
        let mut audio = audio::manager::AudioManager::default();
        seed_track(&mut tracks, &mut audio, 1_000);
        tracks
            .ruler
            .set_screen_rect(Rect::new(0.0, 0.0, 1000.0, 100.0));
        tracks.selection_info = SelectionInfoE::IsSelected(SelectionInfo {
            ix_rng: (100..300).into(),
            screen_x_start: 10.0,
            screen_x_end: 30.0,
        });

        tracks
            .zoom_to_selection_edge(&audio, SelectionEdge::Left)
            .unwrap();

        let ix_range = tracks.ix_range().unwrap();
        assert_eq!(tracks.samples_per_pixel(), Some(0.1));
        assert!((ix_range.start - 50.0).abs() < 1e-9);
        assert!((ix_range.end - 150.0).abs() < 1e-9);
        assert!((tracks.sample_ix_to_screen_x(100.0).unwrap() - 500.0).abs() < 1e-3);
    }

    #[test]
    fn zoom_to_selection_right_edge_centers_edge_and_uses_sample_level_zoom() {
        let mut tracks = Tracks::default();
        let mut audio = audio::manager::AudioManager::default();
        seed_track(&mut tracks, &mut audio, 1_000);
        tracks
            .ruler
            .set_screen_rect(Rect::new(0.0, 0.0, 1000.0, 100.0));
        tracks.selection_info = SelectionInfoE::IsSelected(SelectionInfo {
            ix_rng: (100..300).into(),
            screen_x_start: 10.0,
            screen_x_end: 30.0,
        });

        tracks
            .zoom_to_selection_edge(&audio, SelectionEdge::Right)
            .unwrap();

        let ix_range = tracks.ix_range().unwrap();
        assert_eq!(tracks.samples_per_pixel(), Some(0.1));
        assert!((ix_range.start - 250.0).abs() < 1e-9);
        assert!((ix_range.end - 350.0).abs() < 1e-9);
        assert!((tracks.sample_ix_to_screen_x(300.0).unwrap() - 500.0).abs() < 1e-3);
    }

    #[test]
    fn zoom_to_selection_edge_without_selection_is_noop() {
        let mut tracks = Tracks::default();
        tracks
            .ruler
            .set_screen_rect(Rect::new(0.0, 0.0, 1000.0, 100.0));

        tracks
            .zoom_to_selection_edge(
                &audio::manager::AudioManager::default(),
                SelectionEdge::Left,
            )
            .unwrap();

        assert_eq!(tracks.ix_range(), None);
    }

    #[test]
    fn zoom_to_selection_edge_with_invalid_range_is_noop() {
        let mut tracks = Tracks::default();
        tracks
            .ruler
            .set_screen_rect(Rect::new(0.0, 0.0, 1000.0, 100.0));
        tracks.selection_info = SelectionInfoE::IsSelected(SelectionInfo {
            ix_rng: (100..100).into(),
            screen_x_start: 10.0,
            screen_x_end: 10.0,
        });

        tracks
            .zoom_to_selection_edge(
                &audio::manager::AudioManager::default(),
                SelectionEdge::Left,
            )
            .unwrap();

        assert_eq!(tracks.ix_range(), None);
    }

    #[test]
    fn equal_height_layout_is_enabled_by_default() {
        assert!(Tracks::default().equal_height_layout);
    }

    #[test]
    fn fill_screen_height_only_updates_visible_tracks() {
        let mut tracks = Tracks {
            available_height: 120.0,
            ..Tracks::default()
        };
        let config = TrackConfig::default();
        let mut audio = audio::manager::AudioManager::default();
        let visible_a = insert_buffer(&mut audio, 64);
        let visible_b = insert_buffer(&mut audio, 64);
        let hidden = insert_buffer(&mut audio, 64);

        let visible_a = tracks
            .add_track_to_end(visible_a, TEST_SAMPLE_RATE, &config)
            .unwrap();
        let visible_b = tracks
            .add_track_to_end(visible_b, TEST_SAMPLE_RATE, &config)
            .unwrap();
        let hidden = tracks
            .add_track_to_end(hidden, TEST_SAMPLE_RATE, &config)
            .unwrap();

        tracks.set_track_height(visible_a, 10.0);
        tracks.set_track_height(visible_b, 15.0);
        tracks.set_track_height(hidden, 25.0);
        tracks.set_track_visibility(hidden, false);

        tracks.fill_screen_height(config.min_height).unwrap();

        assert_eq!(tracks.get_track_height(visible_a), Some(60.0));
        assert_eq!(tracks.get_track_height(visible_b), Some(60.0));
        assert_eq!(tracks.get_track_height(hidden), Some(25.0));
    }

    #[test]
    fn fill_screen_height_respects_minimum_total_height() {
        let mut tracks = Tracks {
            available_height: 20.0,
            ..Tracks::default()
        };
        let config = TrackConfig::default();
        let mut audio = audio::manager::AudioManager::default();
        let buffer_id = insert_buffer(&mut audio, 64);
        let track_id = tracks
            .add_track_to_end(buffer_id, TEST_SAMPLE_RATE, &config)
            .unwrap();

        tracks.fill_screen_height(config.min_height).unwrap();

        assert_eq!(
            tracks.get_track_height(track_id),
            Some(config.min_height + crate::model::track::HEADER_HEIGHT)
        );
    }

    #[test]
    fn fill_screen_height_with_no_visible_tracks_is_a_noop() {
        let mut tracks = Tracks {
            available_height: 120.0,
            ..Tracks::default()
        };
        let config = TrackConfig::default();
        let mut audio = audio::manager::AudioManager::default();
        let buffer_id = insert_buffer(&mut audio, 64);
        let track_id = tracks
            .add_track_to_end(buffer_id, TEST_SAMPLE_RATE, &config)
            .unwrap();
        tracks.set_track_height(track_id, 47.0);
        tracks.set_track_visibility(track_id, false);

        tracks.fill_screen_height(config.min_height).unwrap();

        assert_eq!(tracks.get_track_height(track_id), Some(47.0));
    }

    #[test]
    fn equal_height_recalculation_tracks_viewport_and_visible_membership_changes() {
        let mut tracks = Tracks {
            available_height: 120.0,
            ..Tracks::default()
        };
        let config = TrackConfig::default();
        let mut audio = audio::manager::AudioManager::default();
        let buffer_a = insert_buffer(&mut audio, 64);
        let buffer_b = insert_buffer(&mut audio, 64);
        let track_a = tracks
            .add_track_to_end(buffer_a, TEST_SAMPLE_RATE, &config)
            .unwrap();
        let track_b = tracks
            .add_track_to_end(buffer_b, TEST_SAMPLE_RATE, &config)
            .unwrap();

        tracks.fill_screen_height(config.min_height).unwrap();
        assert_eq!(tracks.get_track_height(track_a), Some(60.0));
        assert_eq!(tracks.get_track_height(track_b), Some(60.0));

        tracks.available_height = 180.0;
        tracks.fill_screen_height(config.min_height).unwrap();
        assert_eq!(tracks.get_track_height(track_a), Some(90.0));
        assert_eq!(tracks.get_track_height(track_b), Some(90.0));

        let buffer_c = insert_buffer(&mut audio, 64);
        let track_c = tracks
            .add_track_to_end(buffer_c, TEST_SAMPLE_RATE, &config)
            .unwrap();
        tracks.fill_screen_height(config.min_height).unwrap();
        assert_eq!(tracks.get_track_height(track_a), Some(60.0));
        assert_eq!(tracks.get_track_height(track_b), Some(60.0));
        assert_eq!(tracks.get_track_height(track_c), Some(60.0));

        tracks.set_track_visibility(track_c, false);
        tracks.fill_screen_height(config.min_height).unwrap();
        assert_eq!(tracks.get_track_height(track_a), Some(90.0));
        assert_eq!(tracks.get_track_height(track_b), Some(90.0));
        assert_eq!(tracks.get_track_height(track_c), Some(60.0));

        tracks.remove_track(track_b);
        tracks.fill_screen_height(config.min_height).unwrap();
        assert_eq!(tracks.get_track_height(track_a), Some(180.0));
        assert_eq!(tracks.get_track_height(track_c), Some(60.0));
    }

    #[test]
    fn zoom_to_full_uses_only_visible_tracks() {
        let mut tracks = Tracks::default();
        let config = TrackConfig::default();
        let mut audio = audio::manager::AudioManager::default();
        let short = insert_buffer(&mut audio, 64);
        let long_hidden = insert_buffer(&mut audio, 640);

        let short = tracks
            .add_track_to_end(short, TEST_SAMPLE_RATE, &config)
            .unwrap();
        let long_hidden = tracks
            .add_track_to_end(long_hidden, TEST_SAMPLE_RATE, &config)
            .unwrap();
        tracks.set_track_visibility(long_hidden, false);
        tracks
            .ruler
            .set_screen_rect(Rect::new(0.0, 0.0, 64.0, 100.0));

        tracks.zoom_to_full(&audio).unwrap();

        let visible_track = tracks.get_track(short).unwrap();
        let hidden_track = tracks.get_track(long_hidden).unwrap();
        assert_eq!(tracks.ix_range().unwrap().end, 64.0);
        assert_eq!(
            visible_track.single.sample_rect_raw().unwrap().ix_rng().end,
            64.0
        );
        assert_eq!(
            hidden_track.single.sample_rect_raw().unwrap().ix_rng().end,
            64.0
        );
    }

    #[test]
    fn pan_track_value_range_can_move_below_full_scale_when_linear() {
        let mut tracks = Tracks::default();
        let mut audio = audio::manager::AudioManager::default();
        let track_id = track_with_value_range(
            &mut tracks,
            &mut audio,
            audio::sample::ValRange {
                min: -1.0,
                max: 1.0,
            },
        );

        tracks
            .pan_track_value_range(track_id, -100.0, ValueDisplayScale::default())
            .unwrap();

        let val_rng = tracks
            .get_track(track_id)
            .unwrap()
            .single
            .sample_rect_raw()
            .unwrap()
            .val_rng
            .unwrap();
        assert_eq!(val_rng.min, -3.0);
        assert_eq!(val_rng.max, -1.0);
    }

    #[test]
    fn pan_track_value_range_can_move_below_full_scale_when_skewed() {
        let mut tracks = Tracks::default();
        let mut audio = audio::manager::AudioManager::default();
        let track_id = track_with_value_range(
            &mut tracks,
            &mut audio,
            audio::sample::ValRange {
                min: -1.0,
                max: 1.0,
            },
        );

        tracks
            .pan_track_value_range(track_id, -100.0, ValueDisplayScale { skew_factor: 1.0 })
            .unwrap();

        let val_rng = tracks
            .get_track(track_id)
            .unwrap()
            .single
            .sample_rect_raw()
            .unwrap()
            .val_rng
            .unwrap();
        assert!(val_rng.min < -1.0);
        assert!(val_rng.max <= -1.0);
    }

    #[test]
    fn recenter_track_value_range_restores_full_scale_after_out_of_range_pan() {
        let mut tracks = Tracks::default();
        let mut audio = audio::manager::AudioManager::default();
        let track_id = track_with_value_range(
            &mut tracks,
            &mut audio,
            audio::sample::ValRange {
                min: -1.0,
                max: 1.0,
            },
        );

        tracks
            .pan_track_value_range(track_id, -100.0, ValueDisplayScale::default())
            .unwrap();
        tracks.recenter_track_value_range(track_id).unwrap();

        let val_rng = tracks
            .get_track(track_id)
            .unwrap()
            .single
            .sample_rect_raw()
            .unwrap()
            .val_rng
            .unwrap();
        assert_eq!(
            val_rng,
            audio::sample::ValRange {
                min: -1.0,
                max: 1.0
            }
        );
    }

    #[test]
    fn move_track_reorders_within_order() {
        let mut tracks = Tracks::default();
        let mut audio = audio::manager::AudioManager::default();
        let new_track = |tracks: &mut Tracks, audio: &mut audio::manager::AudioManager| {
            tracks
                .add_track_to_end(
                    insert_buffer(audio, 64),
                    TEST_SAMPLE_RATE,
                    &TrackConfig::default(),
                )
                .unwrap()
        };
        let t0 = new_track(&mut tracks, &mut audio);
        let t1 = new_track(&mut tracks, &mut audio);
        let t2 = new_track(&mut tracks, &mut audio);
        let t3 = new_track(&mut tracks, &mut audio);
        assert_eq!(tracks.tracks_order, vec![t0, t1, t2, t3]);

        // Move the first track to the very end.
        tracks.move_track(t0, 4);
        assert_eq!(tracks.tracks_order, vec![t1, t2, t3, t0]);

        // Move the last track to the front.
        tracks.move_track(t0, 0);
        assert_eq!(tracks.tracks_order, vec![t0, t1, t2, t3]);

        // Dropping into a gap adjacent to the track's own position is a no-op.
        tracks.move_track(t1, 1);
        assert_eq!(tracks.tracks_order, vec![t0, t1, t2, t3]);
        tracks.move_track(t1, 2);
        assert_eq!(tracks.tracks_order, vec![t0, t1, t2, t3]);

        // Unknown track is ignored.
        tracks.remove_track(t2);
        tracks.move_track(t2, 0);
        assert_eq!(tracks.tracks_order, vec![t0, t1, t3]);
    }
}
