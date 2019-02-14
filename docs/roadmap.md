# 2019 Roadmap for druid / piet

There's been a lot of activity in GUI and 2D graphics, but it's not easy to make sense of it all. This document tries to explain what's in place, what the plans are for the next few months, and also a grander dream of what it might become.

## Smaller modules composed in layers

Building a GUI is a large-scale task, and to make any headway it needs to be broken into smaller components. I've factored this work into a number of Rust crates, largely so that each can be worked on and improved separately.

In addition, many of these crates are designed as common infrastructure, and could be used by other GUI efforts.

### druid

### druid-shell

The current scope of druid-shell is similar to that of winit: it pops up a window, and takes inputs. There is one additional set of functionality: it gives access to platform menus.

I didn't use winit because I wanted to experiment, and didn't know whether design decisions of winit would limit performance or functionality. Shortly I want to take a close look at those questions. I can imagine using winit for that common functionality and layering other functionality on top.

### piet and back-ends

I explained the motivation of piet in a [blog post](https://raphlinus.github.io/rust/graphics/2018/10/11/2d-graphics.html). Since then, it's coming along pretty nicely. There are three back-ends: Direct2D which is high performance and Windows-only, Cairo which is mature and portable, but not cleanly packaged as Rust crates and not very performant, and web.

### As yet unnamed high-level text crate

The biggest missing functionality in piet is text handling. What's there right now is very primitive. I think DirectWrite is a good model for what apps need, but of course it's Windows specific.

### As yet unnamed low-level text crate

One of the missing pieces in the Rust ecosystem is a layer for doing what I call low-level text formatting. This involves doing font fallback and applying effects like letterspacing, but not higher level operations like line breaking, hyphenation, bidi, and the representation of rich text.

Once font fallback is resolved, [HarfBuzz] does the actual shaping, and this is nicely packaged.

### kurbo

A very small fraction of the total draw calls in a GUI are *general* paths, but a large fraction are rectangles, and maybe lines and rounded rects as well. The kurbo crate is mostly a representation of shapes, with some geometry calculation as well. It's designed for efficiency, so getting information out of a shape doesn't require any allocation. It also reports when a shape is a simpler, special case, but provides a nice general API, both for producers and consumers of shapes.

## Future: an higher level API surface for UI

A `Widget` in druid is roughly comparable to a `RenderObject` in Flutter. I'm personally comfortable writing UI's directly at this level, but at the same time it might not be ideal. Flutter implements a reactive layer (what it calls Widgets) on top. In my opinion, the best way to do a high level API surface in Rust is an open research question. In additional to elm-style, I can imagine something that resembles imgui, something that expands templates, and something like Interface Builder. There are many interesting details about how state is managed, whether there's interior mutability, whether macros make the syntax sweet or whether it's possible to do something reasonably clean without relying heavily on macros.

So basically I invite the community to explore this.
