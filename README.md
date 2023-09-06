# Anime-dl

[![Crates.io](https://img.shields.io/crates/v/anime-dl?color=orange)](https://crates.io/crates/anime-dl)
[![dependency status](https://deps.rs/repo/github/gabelluardo/anime-dl/status.svg)](https://deps.rs/crate/anime-dl)
![Crates.io](https://img.shields.io/crates/l/anime-dl)

Efficient cli app for downloading anime

### Install

For latest release:

```sh
cargo install anime-dl
```

or for latest commit on `main` branch:

```sh
cargo install --git https://github.com/gabelluardo/anime-dl
```

### Usage

I usually use this:

```sh
adl -D <entries>
```

to store every anime in a properly named directory.

Or for stream after scraped an archive:

```sh
adl -sS <archive> <entries>
```

[![asciicast](https://asciinema.org/a/wdjS4wxIvQrTR7IDLGFW38cM6.svg)](https://asciinema.org/a/wdjS4wxIvQrTR7IDLGFW38cM6)

```
Usage: adl [OPTIONS] [ENTRIES]...

Arguments:
  [ENTRIES]...  Source urls or scraper's queries

Options:
  -d, --dir <DIR>                Root path where store files [default: .]
  -m, --max-concurrent <MAX>     Maximum number of simultaneous downloads allowed [default: 24]
  -r, --range <RANGE>            Episodes to download (es. `1-4` or `1,2,3,4`) [default: 1]
  -S, --site <SITE>              Search anime in remote archive [possible values: aw]
  -D, --default-dir              Save files in a folder with a default name
  -f, --force                    Override existent files
  -a, --anilist-id <ANILIST_ID>  Override app id environment variable [env: ANIMEDL_ID]
  -s, --stream                   Stream episode in a media player
  -i, --interactive              Interactive mode
  -p, --no-proxy                 Disable automatic proxy (useful for slow connections)
      --clean                    Delete app cache
  -h, --help                     Print help
  -V, --version                  Print version
```

For parsing urls from a file (es. `urls`):

```sh
adl [FLAGS] [OPTIONS] $(cat urls)
```

> [!WARNING]
> Streaming requires [mpv](https://mpv.io/) or [vlc](https://www.videolan.org/vlc/)

```sh
adl -s <entries>
```

### Anilist

> [!NOTE]
> For [Anilist](https://anilist.co) integration create an enviroment variable
> `ANIMEDL_ID` with the ID of your [developer api client](https://anilist.co/settings/developer),
> or use the default of the app: `4047`

### Contribution

Currently, there is only an **italian** language scraper, contribution for support other languages are welcome (see [#83](https://github.com/gabelluardo/anime-dl/issues/83)).

### License

Made with 🫶 by **[@gabelluardo](https://github.com/gabelluardo)** in [GPLv3](LICENSE)
