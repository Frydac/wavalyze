## Use new backend/track(model/view)

* [ ] implement zoom to full, after loading wav files


* [x] better cli args
  * [x] ReadConfig::sample_range -> sample::range::OptIxRange
  * [x] add audio::sample::range::OptIxRange
* [x] use new cli args to load files the new way
  * [x] load into audio manager and store resulting file
  * [x] add PathBuf to File
  * [x] add display to file (channels are a bit annoying in debug print), maybe also override debug print
* [x] create new ruler
  * [x] add functionality and test wrt transforming from pixel to sample coordinates and back
    * [ ] test sub-pixel precision
  * [ ] draw grid
    * [ ] extract grit tick generation into the model
      * model/ruler/ix_lattice.rs



* [ ] create new model track instances
* [ ] create new view track instances
* [ ] render the actual new track


## Bugs

* [ ] There needs to be a limit to zooming out, it can cause i64 overflow, probably no need to zoom out that
  far..
    * probably best depend on content, like zoom out  to max 1x or 2x the content, or something
