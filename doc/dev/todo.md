## Use new backend/track(model/view)
* convert/scale PCM16 to PCM24 to/from f32
* □ implement zoom to full, after loading wav files
  * □ communicate zoom to track, sample_rect not updated currently


* ✔ better cli args
  * ✔ ReadConfig::sample_range -> sample::range::OptIxRange
  * ✔ add audio::sample::range::OptIxRange
* ✔ use new cli args to load files the new way
  * ✔ load into audio manager and store resulting file
  * ✔ add PathBuf to File
  * ✔ add display to file (channels are a bit annoying in debug print), maybe also override debug print
* ✔ create new ruler
  * ✔ add functionality and test wrt transforming from pixel to sample coordinates and back
    * □ test sub-pixel precision
  * □ draw grid
    * □ extract grit tick generation into the model
      * model/ruler/ix_lattice.rs



* □ create new model track instances
* □ create new view track instances
* □ render the actual new track


## Bugs

* □ There needs to be a limit to zooming out, it can cause i64 overflow, probably no need to zoom out that
  far..
    * probably best depend on content, like zoom out  to max 1x or 2x the content, or something

* □ when zooming out, take care the samples_per_pixel doesn't overshoot into a negative value (same as above more or
  less)
* □ when loading a file, the tracks should be in order. they were not when e.g.:
    ```bash
    ~/aws/Content/_verified202303/AlainClark/LetSomeAirIn/Auro10_1/48k/Original 🕒 13:36:20 took 53s 
    ❯ wv Ala
    ```
