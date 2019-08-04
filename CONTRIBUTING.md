# How to contribute

We'd love to accept your patches and contributions to this project. There are
just a few small guidelines you need to follow.

## Build
Currently, druid only builds on Windows and macOS. 

Other options may work, but not tested.

#### Windows

run `cargo build`

#### macOS

On macOS, druid requires cairo; see [gtk-rs dependencies] for installation instructions.

You may also need to set your `PKG_CONFIG_PATH`; assuming you have installed `cairo` through homebrew, you can build with,

 ```shell
$> PKG_CONFIG_PATH="/usr/local/opt/libffi/lib/pkgconfig" cargo build
 ```

## Code reviews

All submissions, including submissions by project members, require review. We
use GitHub pull requests for this purpose. Consult [GitHub Help] for more
information on using pull requests.

If your name does not already appear in the [AUTHORS] file, please feel free to
add it as part of your patch.

[gtk-rs dependencies]: http://gtk-rs.org/docs/requirements.html
[GitHub Help]: https://help.github.com/articles/about-pull-requests/
[AUTHORS]: AUTHORS
