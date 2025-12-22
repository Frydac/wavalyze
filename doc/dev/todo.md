## Use new backend/track(model/view)

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
