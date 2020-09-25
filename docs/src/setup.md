# Set up Druid
This tutorial assumes basic familliarity with Rust and a working setup with the basic tooling like
Rustup and Cargo. This tutorial will use stable Rust (v1.39.0 at the time of writing) and the latest
released version of Druid.

This tutorial will first walk you through setting up the dependencies for developing a Druid
application, then it will show you how to set up a basic application, build it and run it.

## Setting up Druid dependencies
In addition to including the `druid` library in your project

### Linux
On Linux, Druid requires gtk+3.

On Ubuntu this can be installed with
```no_compile
sudo apt-get install libgtk-3-dev
```

On Fedora
```no_compile
sudo dnf install gtk3-devel glib2-devel
```

See [gtk-rs dependencies] for more installation instructions.

## Starting a project
Starting a project is as easy as creating an empty application with
```no_compile
cargo new my-application
```
and adding the druid dependency to your Cargo.toml
```no_compile
[dependencies]
druid = "0.6.0"
// or to be on the bleeding edge:
druid = { git = "https://github.com/linebender/druid.git" }
```

[gtk-rs dependencies]: http://gtk-rs.org/docs/requirements.html
