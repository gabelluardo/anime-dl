# Anime-dl

[![Crates.io](https://img.shields.io/crates/v/anime-dl?color=orange)](https://crates.io/crates/anime-dl)
![Crates.io](https://img.shields.io/crates/l/anime-dl)

Efficient cli app for downloading anime

### Install

For latest release:

```sh
cargo install anime-dl
```

or for latest master commit:

```sh
cargo install --git https://github.com/gabelluardo/anime-dl
```

### Usage

I usually use this:
```sh
anime-dl -ac <urls>
```
to store every anime in a properly named directory.

Anyway, the helper is this: 

```
USAGE:
    anime-dl [FLAGS] [OPTIONS] <urls>...

FLAGS:
    -a, --auto           Find automatically output folder name
    -c, --continue       Find automatically last episode (override `-r <range>` option)
    -f, --force          Override existent files
    -h, --help           Prints help information
    -i, --interactive    Interactive choice of episodes
    -O, --one-file       Download only the file form the url (equivalent to `curl -O <url>`)
    -s, --stream         Stream episode in a media player (add -O for single file)
    -V, --version        Prints version information

OPTIONS:
    -d, --dir <dir>...       Root folders where save files [default: .]
    -r, --range <range>      Range of episodes to download
    -S, --search <search>    Search anime in remote archive [possible values: AW, AS]

ARGS:
    <urls>...    Source url
```

For parsing urls from a file (es. `urls`):

```sh
anime-dl [FLAGS] [OPTIONS] $(cat urls)
```

**:warning: Streaming requires `vlc` :warning:**

```sh
anime-dl -sc <urls>...
```

### Known issue

1. Scraper only allows one search at a time.
2. `--` is needed before urls when `-d <dir>...` flag is used.  

### License

GPLv3
