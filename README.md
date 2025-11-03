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
adl download -D <entries>
```

to store every anime in a properly named directory.

Or for stream after scraped an archive:

```sh
adl stream -S <archive> <entries>
```

[![asciicast](https://asciinema.org/a/wdjS4wxIvQrTR7IDLGFW38cM6.svg)](https://asciinema.org/a/wdjS4wxIvQrTR7IDLGFW38cM6)

```
Usage: adl [COMMAND]

Commands:
  stream    Stream anime in a media player
  download  Donwload anime
  clean     Delete app cache
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

For parsing urls from a file (es. `urls`):

```sh
adl [COMMAND] [OPTIONS] $(cat urls)
```

> [!WARNING]
> Streaming requires [mpv](https://mpv.io/) or [vlc](https://www.videolan.org/vlc/)

```sh
adl stream <entries>
```

### Anilist

> [!NOTE]
> For [Anilist](https://anilist.co) integration create an enviroment variable
> `ANIMEDL_ID` with the ID of your [developer api client](https://anilist.co/settings/developer),
> or use the default of the app: `4047`

### Contribution

Currently, there is only an **italian** language scraper, contributions for support other languages are welcome (see [#83](https://github.com/gabelluardo/anime-dl/issues/83)).

#### Development Setup

This project uses [pre-commit](https://pre-commit.com/) hooks to ensure code quality. To set up:

```sh
pip install pre-commit
pre-commit install
pre-commit install --hook-type commit-msg
```

The hooks will automatically run clippy, tests, and validate conventional commit messages.

### License

Made with 🫶 by **[@gabelluardo](https://github.com/gabelluardo)** in [GPLv3](LICENSE)
