# Anime-dl

[![Crates.io](https://img.shields.io/crates/v/anime-dl?color=orange)](https://crates.io/crates/anime-dl)
[![dependency status](https://deps.rs/crate/anime-dl/1.1.1/status.svg)](https://deps.rs/crate/anime-dl/1.1.1)
![Crates.io](https://img.shields.io/crates/l/anime-dl)


Efficient cli app for downloading anime

### Install

For latest release:

``` sh
cargo install anime-dl
```

or for latest master commit:

``` sh
cargo install --git https://github.com/gabelluardo/anime-dl
```

### Usage

I usually use this:

``` sh
anime-dl -ac <entries>
```

to store every anime in a properly named directory.  

Or for stream after scraped an archive:

``` sh
anime-dl -sS <archive> -- <entries>
```

[![asciicast](https://asciinema.org/a/380142.svg)](https://asciinema.org/a/380142)

Anyway, the helper is this: 

``` 
USAGE:
    anime-dl [FLAGS] [OPTIONS] <entries>...

FLAGS:
    -a, --auto           Find automatically output folder name
    -c, --continue       Find automatically last episode (override `-r <range>` option)
        --clean          Delete app cache
    -f, --force          Override existent files
    -h, --help           Prints help information
    -i, --interactive    Interactive mode
    -p, --no-proxy       Disable automatic proxy (useful for slow connections)
    -O, --one-file       Download file without in-app control (equivalent to `curl -O <url>` or `wget <url>`)
    -s, --stream         Stream episode in a media player (add -O for single file)
    -V, --version        Prints version information

OPTIONS:
    -d, --dir <dir>...            Root paths where store files [default: .]
    -m, --max-concurrent <max>    Maximum number of simultaneous downloads allowed [default: 24]
    -r, --range <range>           Range of episodes to download
    -S, --search <site>           Search anime in remote archive [possible values: AW, AS]

ARGS:
    <entries>...    Source urls or scraper's queries
```

For parsing urls from a file (es. `urls` ):

``` sh
anime-dl [FLAGS] [OPTIONS] $(cat urls)
```

**⚠️ Streaming requires `vlc` ⚠️**

``` sh
anime-dl -sc <entries>
```

### Anilist 

For [Anilist](https://anilist.co) integration create an enviroment variable 
`ANIMEDL_ID` with the ID of your [developer api client](https://anilist.co/settings/developer), 
or use the default of the app: `4047`

### Known issue

1. Scraper only allows one search at a time.
2. Stream with [vlc](https://www.videolan.org/vlc/) may not work in Windows

### Contribution 

Feel free to add scrapers for your favorite archive, or make any other kind of contribution. 💪

### License

GPLv3
