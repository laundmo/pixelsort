# Pixelsorter

https://user-images.githubusercontent.com/24855949/188506286-d8f7c826-da83-4e7f-a189-76561d513a82.mp4


Pixelsorting is the visual effect of sorting rows of pixels by some value, it produces a interesting glitchy effect.

This program allows you to pixelsort a image and observe changes made to the parameters in realtime.

# Install

Head to the GitHub [releases](https://github.com/laundmo/pixelsort/releases/latest) and download the executable.

Run the executable, it should work out of the box.

# Usage

Generally you want to adjust the `Threshold Value` to get more or less parts of the image sorted.

There are currently these types of Thresholds and Orderings implemented:

- Luminance: uses the luminance values of the pixel
- ColorSimilarity: uses the distance of the pixel from the provided color.
  - The distance calculation used for this is not quite what i would like this to be. It considers brighter colors to be more similar to everything and darker ones to be less similar.

### Other parameters:

The `Invert` button behind the `Threshold:` dropdown will cause it to match in the other direction - light instead of dark when using Luminance.

For the thresholds, you can set a `merge` value, it defines the pixels between 2 sorting ranges for them to be merged together.

`Extend` can be used to force-extend the threshold ranges in either direction.

The `Revese` button after the `Ordering:` dropdown will reverse the ordered ranges of pixels, light to dark instead of dark to ligth when using luminance.

## ToDo

- Exporting images
- Multiple sorting stages - allow a sort to be applied on top of another sort.
- More Threshold/Ordering types (If you have a idea for one, make a Issue!)
