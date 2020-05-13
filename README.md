# Animeworld-dl

Efficient cli app for downloading anime

### Install

```
cargo install --git https://github.com/gabelluardo/animeworld-dl
```

or build the binary form source and install it in `.cargo/bin` folder:

```
git clone https://github.com/gabelluardo/animeworld-dl
cd animeworld-dl

cargo build --release
cargo install --path .
```

### Usage

```
USAGE:
    animeworld-dl [FLAGS] [OPTIONS] <urls>...

FLAGS:
    -a, --auto               Find automatically output folder name
    -c, --continue           Find automatically last episode (this overrides `-e` option)
    -F, --finished           Mark anime as finished [WIP]
    -f, --force              Override existent files
    -h, --help               Prints help information
    -S, --single-download    Download only the file form the url (equivalent to `curl -O <url>`)
    -V, --version            Prints version information

OPTIONS:
    -d, --dir <dir>...                 Path folder where save files [default: .]
    -e, --end <end>                    Last episode to download [default: 0]
    -M, --max-threads <max-threads>    Max number of concurrent downloads [default: 32]
    -s, --start <start>                First episode to download [default: 1]

ARGS:
    <urls>...    Source url
```

For parsing urls from a source file (es. `urls`):

```
animeworld-dl [FLAGS] [OPTIONS] $(cat urls)
```

### License

GPL v3
