# Setting up Druid and starting a project
This tutorial assumes basic familliarity with Rust and a working setup with the basic tooling like
Rustup and Cargo. This tutorial will use stable Rust (v1.39.0 at the time of writing) and the latest
released version of Druid.

This tutorial will first walk you through setting up the dependencies for developing a Druid
application, then it will show you how to set up a basic application, build it and run it.

## Setting up Druid dependencies
In addition to including the druid library in your project 

#### macOS
On macOS, druid requires [cairo]; if you use homebrew, `brew install cairo`
should be sufficient. Removing this dependency is on the roadmap.

#### Linux
On Linux, druid requires gtk+3.

|
|---|---|
|Ubuntu|```sudo apt install gtk3-dev```|
|Fedora pre 21||
|Fedora post 21||

See [gtk-rs dependencies] for more installation instructions.


## Starting a project
Starting a project is as easy as creating an empty application with
```
cargo new my-application
``` 
and adding the druid dependency to your Cargo.toml
```
[dependencies]
druid = "0.4.0"
```



[gtk-rs dependencies]: http://gtk-rs.org/docs/requirements.html
