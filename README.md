# Anime-dl

[![Crates.io](https://img.shields.io/crates/v/anime-dl?color=orange)](https://crates.io/crates/anime-dl)
[![dependency status](https://deps.rs/repo/github/gabelluardo/anime-dl/status.svg)](https://deps.rs/crate/anime-dl)
![Crates.io](https://img.shields.io/crates/l/anime-dl)


Efficient cli app for downloading anime

### Install

For latest release:

``` sh
cargo install anime-dl
```

or for latest commit on `main` branch:

``` sh
cargo install --git https://github.com/gabelluardo/anime-dl
```

### Usage

I usually use this:

``` sh
adl -cD <entries>
```

to store every anime in a properly named directory.  

Or for stream after scraped an archive:

``` sh
adl -sS <archive> <entries>
```

[![asciicast](https://asciinema.org/a/392118.svg)](https://asciinema.org/a/392118)

Anyway, the helper is this: 

``` 
USAGE:
    adl [OPTIONS] [ENTRIES]...

ARGS:
    <ENTRIES>...    Source urls or scraper's queries

OPTIONS:
    -a, --anilist-id <ANILIST_ID>    Override app id environment variable [env: ANIMEDL_ID]
    -c, --continue                   Find automatically last episode
        --clean                      Delete app cache
    -d, --dir <DIR>                  Root paths where store files [default: .]
    -D, --default-dir                Save files in a folder with a default name
    -f, --force                      Override existent files
    -h, --help                       Print help information
    -i, --interactive                Interactive mode
    -m, --max-concurrent <max>       Maximum number of simultaneous downloads allowed [default: 24]
    -p, --no-proxy                   Disable automatic proxy (useful for slow connections)
    -r, --range <range>              Episodes to download (es. `1-4` or `1,2,3,4`) [default: 1]
    -s, --stream                     Stream episode in a media player
    -S, --site <site>                Search anime in remote archive [possible values: aw]
```

For parsing urls from a file (es. `urls`):

``` sh
adl [FLAGS] [OPTIONS] $(cat urls)
```

**⚠️ Streaming requires [mpv](https://mpv.io/) or [vlc](https://www.videolan.org/vlc/) ⚠️**

``` sh
adl -sc <entries>
```

### Anilist 

For [Anilist](https://anilist.co) integration create an enviroment variable 
`ANIMEDL_ID` with the ID of your [developer api client](https://anilist.co/settings/developer), 
or use the default of the app: `4047`


### Contribution 

Currently, there is only an _italian_ language scraper, feel free to add others ([#83](https://github.com/gabelluardo/anime-dl/issues/83)) for your favorite archive, or to make any other kind of contribution. 💪

### License

[GPLv3](LICENSE)
