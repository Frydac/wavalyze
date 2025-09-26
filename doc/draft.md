## Diff functionality
  * at least start with cli options to select 2 tracks from 1 or 2 files
  * super position tracks with 2 colors in some way
    * when hovering, show diff value too


## TODO
* rulers
  * sample index/time
  * sample value
* zoom sample value, y-axis
* zoom rectangle

* selection
  * when clicked
  * when dragged

* info panel
  * tab?


* steal ideas from Audacity/Audition :D

* should use 1 zoom level for all tracks?
  * now we zoom/shift independently, float calculations could diverge?
  * handy for sample index ruler



## maybe use 2d transformation matrices?
  * scale
    * x
      * factor = pixels per sample
  * translate
    * x
      * after scale so we translate with nr of pixels

  * operations on scale/translate x
    * shift left/right
      * adjust translate x
    * make selection
      * need to be able to invert from pixels coordinates to sample coordinates
        * so invert translate
        * invert scale
    * zoom in/out around mouse position
        * need to adjust both translate and scale
        * example
          * scale = 2
          * translate = 0
          * -1 -1 | 0 0 1 1 2 2 | 3 3
        


## shift

  * given:
    * a sample rect, we only use the sample index range
    * a pixel offset
    * a zoom level
    * a shift in pixels
  * we want to shift the sample rect by the shift in pixels
  * the zoom level is the number of pixels per sample
  * the pixel offset is the number of pixels to shift the sample rect by


## Drawing samples

* when we are drawing single samples, we want to be able to shift by pixels, possibly less than samples
  * so we want to store:
    * which sample is at the 0 position, to align different tracks
    * the zoom level (pixels per sample)
        * above 2 parameters give us a 'sample' coordinate system
    * shift in pixels wrt to 0 position
      * could be very large when zoomed in on a large file
      * alternatively shift in samples + offset in pixels
    * the screen rect (pixels available for drawing)
  * these give use the ability to position the visible samples in the view buffer
    * sample @ 0 pos + zoom level:
      * pixel position of samples in the 'sample' coordinate system
    * shift in pixels gives us a transformation from 'sample' to 'pixel' coordinate system
      * also use the screen width to know how many pixels and samples to draw
    * we can do the above 2 in one go i think
    * the actual screen rect gives us a transformation from 'pixel' to 'screen' coordinate system

e.g.
  pixels_per_sample = 10
  sample_at_zero = 10 (relative shift)
  shift_in_pixels = 15
  screen rect = width = 100

  sample ix 1 = (1 - 10) * 10 + 15 = pixel ix -85
  sample ix 2 = (2 - 10) * 10 + 15 = pixel ix -75





## hovering

view.update
  * each track ui
    * hovering.ui
      * 




## Reduce data for drawing

A monitor is upto 4k pixels wide, a 1 min audio file @ 48kHz = 60 * 48000 = 2.880.000 samples.
Which is 2.880.000 / 4k = 720 samples per pixel.
Drawing 720 pixels in the width of 1 pixel we are essentially drawing a line from the min to the max value of those
720 samples.
So to reduce the draw calls, we can search for the min/max values of those samples and draw a line from min to max,
right on the pixel column.

How to do this, example:
say we have 10 samples and 3 pixels

ratio = 10 / 3 = 3.33

determine which samples fall into which pixel
pixel_x = sample_ix / ratio

nr samples     nr_pixels_x    ratio          sample_ix      six/ratio      pixel_ix
10             3              3.333333333    0              0              0
                                             1              0.3            0
                                             2              0.6            0
                                             3              0.9            0
                                             4              1.2            1
                                             5              1.5            1
                                             6              1.8            1
                                             7              2.1            2
                                             8              2.4            2
                                             9              2.7            2

Then for each 'chunk' we do a min/max and store that, and use that to draw line segments? we don't want a linestrip.
May need to research how to do this more efficiently.

When ratio < 1, this means we have more pixels than samples, then we draw pixels like Audacity?
a dot per sample, and then a vertical line from the sample to the midline, i.e. from the sample (x, y) to (x, 0)

## Zoom level
* define as: "samples per pixel"
  * this makes the zoom independent of the window size
  * Then use the available pixel width (pixw) to determine how to fill the view buffer
    * the view buffer should be as wide as the nr of samples that fit in the pixw
    * this buffer holds mostly integer indices:
      * samples < pixels: each sample is at a certain pixel, draw line to (x, 0)
      * samples > 2 * pixels: for each pixel we should have exactly 2 positions:
        * min and max of all the samples that fall into that pixel
        * we draw a line from min to max
        * don't draw the line to the next pixel?
      * else we just draw the line through all the samples that fit in the pixw
* The horizontal zoom may depend on the window size? yeah I think so


* flow
  * start app
    * show menu and empty area for tracks
      * create model
      * create view, and pass model (move it? by reference? I feel like it shouldn't own the model)
        * view contains all the ui elements in startup code
    * via open dialog open a wav file
       * Model::add_file(path)


## Serialization
* learn about serde
* use it to store the model?
* how about version management and supporting older versions?
