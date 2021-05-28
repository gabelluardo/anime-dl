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

or for latest master commit:

``` sh
cargo install --git https://github.com/gabelluardo/anime-dl
```

### Usage

I usually use this:

``` sh
anime-dl -cD <entries>
```

to store every anime in a properly named directory.  

Or for stream after scraped an archive:

``` sh
anime-dl -sS <archive> -- <entries>
```

[![asciicast](https://asciinema.org/a/392118.svg)](https://asciinema.org/a/392118)

Anyway, the helper is this: 

``` 
USAGE:
    anime-dl [FLAGS] [OPTIONS] <entries>... --range <range>

FLAGS:
    -D, --default-dir    Save files in a folder with a default name
    -c, --continue       Find automatically last episode
        --clean          Delete app cache
    -f, --force          Override existent files
    -h, --help           Prints help information
    -i, --interactive    Interactive mode
    -p, --no-proxy       Disable automatic proxy (useful for slow connections)
    -s, --stream         Stream episode in a media player
    -V, --version        Prints version information

OPTIONS:
    -a, --animedl-id <animedl-id>    Override app id environment variable [env: ANIMEDL_ID]
    -d, --dir <dir>...               Root paths where store files [default: .]
    -m, --max-concurrent <max>       Maximum number of simultaneous downloads allowed [default: 24]
    -r, --range <range>              Episodes to download (es. `1-4` or `1,2,3,4`) [default: 1]
    -S, --search <site>              Search anime in remote archive [possible values: AW, AS]

ARGS:
    <entries>...    Source urls or scraper's queries
```

For parsing urls from a file (es. `urls` ):

``` sh
anime-dl [FLAGS] [OPTIONS] $(cat urls)
```

**⚠️ Streaming requires [vlc](https://www.videolan.org/vlc/) ⚠️**

``` sh
anime-dl -sc <entries>
```

### Anilist 

For [Anilist](https://anilist.co) integration create an enviroment variable 
`ANIMEDL_ID` with the ID of your [developer api client](https://anilist.co/settings/developer), 
or use the default of the app: `4047`


### Contribution 

Currently there is only an italian language scraper, feel free to add others for your favorite archive, or to make any other kind of contribution. 💪

### License

[GPLv3](LICENSE)
