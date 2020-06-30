# Druid mdBook documentation
This folder contains Druid documentation in mdBook format. mdBook allows documentation written in
markdown to be published as html. This README.md gives some pointers for contributers on how to
edit, preview and publish the documentation.

## Editing
mdBook handles writing documentation in a similar way to writing software. Documentation 'source
code' lives in the `docs/src` folder in the form of markdown files. It can be built and published
as html using the mdBook tool.
To edit documentation you edit the corresponding markdown file in `docs/src`. The
`docs/src/SUMMARY.md` file contains the index for the documentation with links to files for all the
chapters.

## Preview documentation
To preview the documentation or to host it on your own system for offline viewing the mdBook tool
needs to be installed. The easiest way to install it is from the crates.io repository using cargo.
`cargo install mdbook`

After this you can start mdBook to serve the documentation locally using
`mdbook serve` from the `docs\` directory. This will serve documentation on `http://localhost:3000`

## Publish documentation
To publish documentation to github pages the documentation needs to be built as html and then moved
to the `gh-pages` branch. This can be done manually or by the build server.
To build the documentation from the project root run;
`mdbook build docs`
This will build the documentation to the `docs\book` folder. This folder can then be copied onto the
`gh-pages` branch. This will tell github to publish the documentation. For the Druid repository it
will be hosted on [https://linebender.org/druid/]
