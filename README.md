# Animeworld-d

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
    -c, --continue    progress unless episode exist [WIP]
    -f, --finished    mark anime as finished [WIP]
    -h, --help        Prints help information
    -V, --version     Prints version information

OPTIONS:
    -d, --dir <dir>...     path folder where save files [default: .]
    -s, --start <start>    where start the downloads [default: 1]

ARGS:
    <urls>...    source url
```

### License

GPL v3