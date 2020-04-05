# Animeworld-dl

Efficient cli app for downloading anime

### Install

Build the binary form source ans install it in `.cargo/bin` folder:

```
git clone https://github.com/gabelluardo/animeworld-dl
cd animeworld-dl

cargo build --release
cargo install --path .
```

### Usage

```
USAGE:
    animeworld-dl [FLAGS] [OPTIONS] [--] [urls]...

FLAGS:
    -c, --continue    Find automatically last episode
    -f, --finished    Mark anime as finished [WIP]
    -h, --help        Prints help information
    -V, --version     Prints version information

OPTIONS:
    -d, --dir <dir>...                 Path folder where save files [default: .]
    -e, --end <end>                    Last episode to download [default: 0]
    -M, --max-threads <max-threads>    Max number of thread [default: 32]
    -s, --start <start>                First episode to download [default: 1]

ARGS:
    <urls>...    Source url
```

### License

GPL v3
