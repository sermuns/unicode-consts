# `unicode-consts`

I just want to find a symbol without having to open a browser and copy-pasting from there...

This is a library providing constant `&str`'s for every unicode character, organized into modules per unicode block.

## TODO

- [ ] add as many `#[doc(alias = ...)]` as possible for both modules and constants.
- [ ] figure out what to do with `0000`..`001F`, '`<control>`'-characters. currently they are ignored.
- [ ] (possibly add feature flags per unicode block?)
