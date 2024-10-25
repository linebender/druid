# UNMAINTAINED

As the Druid project has been discontinued, **we will not be accepting any more contributions**.

Please take a look at some of our other projects instead, especially the Druid successor [Xilem].

# How to contribute

The following document reflects the state of contribution policy before the project was discontinued.

## Changelog

Every pull request should document all changes made in the [changelog].
- The format is always `- [what changed]. ([#pr-number] by [@username])`.
- The number can be guessed or added after creating the pull request.
- Links to all pull requests are recorded at the bottom of the unreleased section.
- If you are a new contributor, please add your name to the others at the bottom of the CHANGELOG.md.

If your name does not already appear in the [AUTHORS] file, please feel free to
add it as part of your patch.

## Code reviews

All submissions, including submissions by project members, require review. We
use GitHub pull requests for this purpose. Consult [GitHub Help] for more
information on using pull requests.


## Before opening a PR

Testing a patch on github can take 15+ minutes, and you can save a lot of time by
testing locally. We recommend using the following [git `pre-push` hook], by
copying it to `druid/.git/hooks/pre-push`:

```sh
#!/bin/sh

set -e

echo "cargo fmt"
cargo fmt --all -- --check
echo "cargo clippy druid-shell"
cargo clippy --manifest-path=druid-shell/Cargo.toml --all-targets --features=raw-win-handle -- -D warnings
echo "cargo clippy druid"
cargo clippy --manifest-path=druid/Cargo.toml --all-targets --features=svg,image,im,raw-win-handle -- -D warnings
echo "cargo clippy druid (wasm)"
cargo clippy --manifest-path=druid/Cargo.toml --all-targets --features=image,im --target wasm32-unknown-unknown -- -D warnings
echo "cargo clippy druid-derive"
cargo clippy --manifest-path=druid-derive/Cargo.toml --all-targets -- -D warnings
echo "cargo clippy book examples"
cargo clippy --manifest-path=docs/book_examples/Cargo.toml --all-targets -- -D warnings
echo "cargo test druid-shell"
cargo test --manifest-path=druid-shell/Cargo.toml
echo "cargo test druid"
cargo test --manifest-path=druid/Cargo.toml --features=svg,image,im
echo "cargo test druid-derive"
cargo test --manifest-path=druid-derive/Cargo.toml
echo "cargo test book examples"
cargo test --manifest-path=docs/book_examples/Cargo.toml
```

# How to maintain

## Preparing for a new release

If you're already contributing to this project and want to do more,
then there might be a chance to help out with the preparation of new releases.
Whether you're new or have prepared Druid releases many times already,
it helps to follow a checklist of what needs to be done. This is that list.

### Increasing the versions

The `druid`, `druid-shell`, and `druid-derive` `Cargo.toml` files need to be updated.
The `version` field needs to be increased to the next [semver] version that makes sense.
These packages all also import each other and those cross-dependency versions need updating too.

You should also search for the previous version number across the whole workspace
to find any other references that might need updating. There are for example plenty of links
to specific version documentation that will need updating.

### Changelog

- Add a new *Unreleased* section by copying the current one.
  Keep the sections, but delete the entries.
- Rename the old *Unreleased* section to the target release version and add the release date.
- Add the correct link for the target release revision to the bottom of the file.
- Update the changelog introduction message to reflect this new release.
- Delete any empty sections.
- Tidy up the entries, possibly reordering some for more logical grouping.

### Dependencies

**We only test and specify the newest versions of dependencies.** Read on for more details.

Rust dependencies like Druid specify their own sub-dependencies in `Cargo.toml`.
These specifications are usually version ranges according to [semver],
stating that the dependency requires a sub-dependency of the specified version
or any newer version that is still compatible. It is up to the final application
to choose which actual versions get used via the `Cargo.lock` file of that application.

Because the final application chooses the sub-dependency versions and they are most likely
going to be higher than the minimum that is specified in our `Cargo.toml` file,
we need to make sure that Druid works properly with these newer versions.
Yes according to [semver] rules they should work, but library authors make mistakes
and it won't be a good experience or a sign of Druid's quality if a new developer
adds Druid as a dependency and it won't even compile.
For that reason our CI testing always uses the highest version that is still compatible.
This mimics what a new developer would experience when they start using Druid.

What about the minimum supported version or all the versions between the minimum and maximum?
It is not practical for us to test all the combinations of possible sub-dependency versions.
Without testing there can easily be mistakes. Let's say our `Cargo.toml` specifies that
we depend on the package `foo` version `^1.1.1` and the latest `foo` version is `1.1.3`.
The CI tests with `1.1.3` and contributors have `1.1.3` in their local `Cargo.lock`.
`Cargo.toml` specifies `1.1.1` as the minimum because that was the latest version
when the dependency was added and `1.1.1` did work just fine originally.
However it turns out that this specific version had a bug which doesn't interact well
with some other package `bar`. Our CI testing or manual testing would never find this out,
because we're already using `1.1.3` and deleting and regenerating `Cargo.lock` wouldn't change that.
Just because `1.1.1` used to work back in the day doesn't mean that it will always keep working.

One partial solution to this problem is to be more precise in what we are actually promising.
So whenever we release a new version we also update all our dependencies in `Cargo.toml`
to match the versions that we are actually testing with. This will be much more accurate
to the spirit of the version specification - Druid will work with the specified version
and any newer one if it's [semver] compatible. We're not testing the extremely big matrix of
old versions of our sub-dependencies and so we shouldn't claim that the old versions will work.

#### Prerequisites for updating the dependency specifications

An easy way to do this is to use the `cargo upgrade` tool available via [`cargo-edit`].

```sh
cargo install cargo-edit
```

*Note that the following instructions were written for `cargo-edit` v0.11. If you have a newer
major version there may have been changes to the way `cargo-edit` works.*

#### Performing the update

We want to update our `Cargo.toml` files to specify the newest versions which are still
[semver] compatible with what we have previously specified in our `Cargo.toml` files.

If you just want to see what would happen you can add the `--dry-run` option.

```sh
cargo upgrade
```

Running this command will update all the `Cargo.toml` files in the workspace and also `Cargo.lock`.

#### Semver incompatible updates

Incompatible version updates should be done manually after carefully reviewing the changes.
However you can still use the `cargo upgrade` tool to find out which dependencies could be updated.

```sh
cargo upgrade -i --dry-run
```

Then based on the reported potential updates you should manually go and check out what has changed,
plus how and if it makes sense to update to the newer version.

### Screenshots

We need to update the screenshots. This involves:

- Taking new screenshots.
- Adding them to the repository in the `screenshots` branch on Github.
- Updating the links in the README to point to the new screenshots.

### Publishing

Once all the docs, cargo files and screenshots have been updated, we need to
publish the crate to crates.io.

#### Ensure your git setup supports symbolic links

Druid makes use of symbolic links but not every git configuration has symlink support enabled,
[most commonly on Windows](https://github.com/git-for-windows/git/wiki/Symbolic-Links).
Make sure your configuration has it enabled. The following command must not return `false`.

```sh
git config core.symlinks
```

If the above command returns `false` then you need to enable this setting and reset your clone.

##### Enabling git supprot for symbolic links on Windows

First you need to make sure your user account has the `SeCreateSymbolicLinkPrivilege` privilege.
Easiest test for this is to open `cmd.exe` and run the following:

```sh
echo "Hello" > original.txt
mklink link.txt original.txt
type link.txt
```

If the `mklink` command didn't complain about privileges and the final `type` command prints out
*Hello* then you have the privilege. If there is a failure, then you need to enable the privilege.

There are [many ways to enable this privilege](https://github.com/git-for-windows/git/wiki/Symbolic-Links#allowing-non-administrators-to-create-symbolic-links),
but we'll cover two here. First option is to just enable *developer mode* from the settings app.
This is a bit heavyweight and changes a lot of settings but it will work. Another more precise
option is to run `gpedit.msc` as Administrator, navigate to *Computer Configuration >
Windows Settings > Security Settings > Local Policies > User Rights Assignment* and add your
account name to the *Create symbolic links* policy. Then you will need to log out and back in.

Once you have the system privilege you must change your git system configuration, which you can
do by executing the following command as Administrator:

```sh
git config --system core.symlinks true
```

Then you must also change the configuration of your local clone of the Druid repository,
by executing the following commands in the repo directory:

```sh
git config --local core.symlinks true
git reset --hard
```

Now you should be all set.

#### Publishing the crates

We need to publish the crates in a specific order, because the `druid` crate depends on the two
other crates. We use `--no-verify` for `druid-derive` to deal with a cyclic dependency on the
`druid` version which we haven't published yet.

```sh
cargo publish --manifest-path="druid-derive/Cargo.toml" --no-verify
cargo publish --manifest-path="druid-shell/Cargo.toml"
cargo publish --manifest-path="druid/Cargo.toml"
```

#### Tagging the release

Once we've published our crate, we create a new git tag:

```sh
git tag $MY_NEW_VERSION_NUMBER HEAD
```

Finally, we [create a new Github release](https://docs.github.com/en/github/administering-a-repository/releasing-projects-on-github/managing-releases-in-a-repository#creating-a-release).

[GitHub Help]: https://help.github.com/articles/about-pull-requests/
[AUTHORS]: AUTHORS
[changelog]: CHANGELOG.md
[`cargo-edit`]: https://crates.io/crates/cargo-edit
[semver]: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html
[git `pre-push` hook]: https://githooks.com
[Xilem]: https://github.com/linebender/xilem
