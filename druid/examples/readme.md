# Examples

There are some diferent kind of examples, some specifically show of one 
part of druid, and others show off more complete examples of how to tie everything together. The later are listed seperatly under [showcases](##Showcases)  

## Anim
```
cargo run --example anim
```
This example shows how to make a simple animation using `Event::AnimFrame`.

## Async Event
```
cargo run --example async_event
```
Having a long function running in the background feeding in new data is important to not block the UI from running. This is similair to [blocking function](##Blocking Function) but here the function runs for the lifetime of the program. 

## Blocking Functions
```
cargo run --example blocking_functions
```
Sometimes you need to fetch some data from disk or from the internet, but you should never block the UI thread with long running operations! Instead you should spawn of a new thread (or something liks async functions) and have it send you the data back like here. This is very similair to [async event](##Async Event) except the lifetime of the thread is shorter here.

## Cursor
```
cargo run --example cursor
```
Setting the cursor gives a lot of context to the user. Like the cursor is diferent when selcting text compared to hovering over a button. The way to handle cursors is a bit finicky at the moment, you have to set it every mousemove event and when you want to set it.

## Custom Widget
```
cargo run --example custom_widget
```
This shows how to use all of the methods on `PaintCtx` used for drawing on a canvas. You can use this to draw everything from text to images to curves.

## Either
```
cargo run --example either
```
It is very usefull to hide some UI based on a condition. This widget hides one of the 2 children. This can be usefull for loading screens or a click-to-reveal like feature. 


## Showcases

### Calc
```
cargo run --example calc
```

This is a showcase of a simple calculator. There are better ways to implement the calculator logic, but it provides all the standard operations like adition devision multiplication C and CE.

### Event Viewer
```
cargo run --example event_viewer
```

This shows you how to capture most events and you can see what gives which event with what data. You can compare mouse clicks to keyboard typing and see how modifiers, like shift and ctrl, are handled.