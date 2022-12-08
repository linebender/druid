# Resolution independence

## What is a pixel anyway?

Pixel is short for *picture element* and although due to its popularity
it has many meanings depending on context, when talking about pixels in the context of druid
a pixel means always only one thing. It is **the smallest configurable area of color
that the underlying platform allows `druid-shell` to manipulate**.

The actual physical display might have a different resolution from what the platform knows or uses.
Even if the display pixel resolution matches the platform resolution,
the display itself can control even smaller elements than pixels - the sub-pixels.

The shape of the physical pixel could be complex and definitely varies from display model to model.
However for simplicity you can think of a pixel as a square which you can choose a color for.

## Display pixel density

As technology advances the physical size of pixels is getting smaller and smaller.
This allows display manufacturers to put more and more pixels into the same sized screen.
The **pixel densities of displays are increasing**.

There is also an **increasing variety in the pixel density** of the displays used by people.
Some might have a brand new *30" 8K UHD* (*7680px \* 4320px*) display,
while others might still be rocking their *30" HD ready* (*1366px \* 768px*) display.
It might even be the same person on the same computer with a multi-display setup.

## The naive old school approach to UI

For a very long time UIs have been designed without thinking about pixel density at all.
People tended to have displays with roughly similar pixel densities, so it all kind of
worked most of the time. However **it breaks down horribly** in a modern world.
The *200px \* 200px* UI that looks decent on that *HD ready* display is barely visible
on the *8K UHD* display. If you redesign it according to the *8K UHD* display then
it won't even fit on the *HD ready* screen.

## Platform specific band-aids

Some platforms have mitigations in place where that small *200px \* 200px* UI
will get scaled up by essentially **taking a screenshot of it and enlarging the image.**
This will result in a blurry UI with diagonal and curved lines suffering the most.
There is more hope with fonts where the vector information is still available to the platform,
and instead of scaling up the image the text can be immediately drawn at the larger size.

## A better solution

The application should draw everything it can with **vector graphics**,
and have **very large resolution image** assets available where vectors aren't viable.
Then at runtime the application should identify the display pixel density
and resize everything accordingly. The vector graphics are easy to resize and
the large image assets would be scaled down to the size that makes sense for the specific display.

## An even better way

Druid aims to make all of this as **easy and automatic** as possible.
Druid has expressive vector drawing capabilities that you should use whenever possible.
Vector drawing is also used by the widgets that come included with druid.
Handling different pixel densities is done at the `druid-shell` level already.
In fact pixels mostly don't even enter the conversation at the `druid` level.
The `druid` coordinate system is instead measured in **display points** (**dp**),
e.g. you might say a widget has a width of **100dp**.
*Display points* are conceptually similar to Microsoft's *device-independent pixels*,
Google's *density-independent pixels*, Apple's *points*, and CSS's *pixel units*.

You **describe the UI using display points and then druid will automatically
translate that into pixels** based on the pixel density of the platform.
Remember there might be multiple displays connected with different pixel densities,
and your application might have multiple windows - with each window on a different display.
It will all just work, because druid will adjust the actual pixel dimensions
based on the display that the window is currently located on.

## High pixel density images with druid

*TODO: Write this section after it's more clear how this works and if its even solved.*