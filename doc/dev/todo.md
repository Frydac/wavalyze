## Bugs

* [ ] There needs to be a limit to zooming out, it can cause i64 overflow, probably no need to zoom out that far..
    * probably best depend on content, like zoom out  to max 1x or 2x the content, or something
* [ ] when zooming out, take care the samples_per_pixel doesn't overshoot into a negative value (same as above more or
  less)
* [ ] when zooming in, limit the max zoom level, and make sure it doesn't pan when we get there (like it does now)
* [ ] when pannnig past the end of the waveform, it still draws some of it when zoomed out (minmax view)

* [x] when loading a file, the tracks should be in order. they were not when e.g.:
    ```bash
    ~/aws/Content/_verified202303/AlainClark/LetSomeAirIn/Auro10_1/48k/Original
    ❯ wv Ala
    ```


## Performance ideas

* [ ] when many tracks, determine which are visible in the scroll area, and only update the view buffer for those
  * [ ] do keep storing the screen/sample rects, but not update the view buffer, when then the track comes back into the scroll
    area that is visible, it should have all the info it needs to render properly
* [ ] do lazy creation of thumbnail/cache levels?
    though it seems that creating the thumbnail levels is significantly faster than actually reading the data from the file
    might be not worth it
* [ ] don't create thumbnail level data with SampleType s, it has no use and need to convert it to pixel positions anyway
  * [ ] maybe store LevelData with pixels in 'canonical' format, with pixel positions in a screen_rect that has y range
    [-1.0, 1.0] and x range [0.0, nr_samples / samples_per_pixel] for the y range?
* [ ] maybe it helps to add more thumbnail levels dynamically when fps is low?
  * [ ] would need to detect when the fps is low, and then add more levels
* [ ] reduce allocations during rendering
  * [ ] when creating the labels for the rulers, it re-allocates the string for each label each time it is rendered


## Features

### Use new backend/track(model/view)

* [x] convert/scale PCM16 to PCM24 to/from f32
* [x] implement zoom to full, after loading wav files
  * [x] communicate zoom to track, sample_rect not updated currently
* [x] better cli args
  * [x] ReadConfig::sample_range -> sample::range::OptIxRange
  * [x] add audio::sample::range::OptIxRange
* [x] use new cli args to load files the new way
  * [x] load into audio manager and store resulting file
  * [x] add PathBuf to File
  * [x] add display to file (channels are a bit annoying in debug print), maybe also override debug print
* [x] create new ruler
  * [x] add functionality and test wrt transforming from pixel to sample coordinates and back
    * [x] test sub-pixel precision
  * [x] draw grid
    * [x] extract grid tick generation into the model
      * model/ruler/ix_lattice.rs
* [x] create new model track instances
* [x] render the actual new track

