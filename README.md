# xi-win

## Xi editor for Windows

This project is currently a seedling, which I am hopeful will grow
into a useful front-end for [xi editor](https://github.com/google/xi-editor).

It makes some decisions which are fairly unusual for GUI software
written in 2017. It targets winapi directly, rather than using a
toolkit, and is written in Rust. The main reason is so that we can
target performance; winapi has api's for Direct2D rendering that may
enable significantly lower latency and smoother scrolling than
accessible through a toolkit. Part of the reason for taking on this
project is to quantify the relative performance.

Other important dimensions of performance are startup speed and
executable size.

Given the goals of this project and the (current) flux in the xi
protocol, I expect the focus of this project to be input and rendering
pipelines, with less emphasis on features; it may be a while before
this is really useful as an editor. That said, I certainly welcome
any help in getting there sooner.

## Contributions

We gladly accept contributions via GitHub pull requests. Please see CONTRIBUTING.md for more details.

If you are interested in contributing but not sure where to start, there is an active IRC channel at #xi on irc.mozilla.org. There is also a subreddit at /r/xi_editor.

## Authors

The main author is Raph Levien.
