# sudachiclone-rs - SudachiPyClone by rust

[![sudachiclone at crates.io](https://img.shields.io/crates/v/sudachiclone.svg)](https://crates.io/crates/sudachiclone)
[![sudachiclone at docs.rs](https://docs.rs/sudachiclone/badge.svg)](https://docs.rs/sudachiclone)
[![Actions Status](https://github.com/Yasu-umi/sudachiclone-rs/workflows/test/badge.svg)](https://github.com/Yasu-umi/sudachiclone-rs/actions)

sudachiclone-rs is a Rust version of [Sudachi](https://github.com/WorksApplications/sudachi), a Japanese morphological analyzer.

## Install CLI

### Setup.1 Install sudachiclone

sudachiclone is distributed from [crates.io](https://crates.io/crates/sudachiclone). You can install sudachiclone by executing cargo install sudachiclone from the command line.

```bash
$ cargo install sudachiclone
```

### Setup2. Install dictionary

The default dict package SudachiDict_core is distributed from WorksAppliations Download site. Run pip install like below:

```bash
$ pip install https://object-storage.tyo2.conoha.io/v1/nc_2520839e1f9641b08211a5c85243124a/sudachi/SudachiDict_core-20200127.tar.gz
```

## Usage CLI

After installing sudachiclone, you may also use it in the terminal via command sudachiclone.

You can excute sudachiclone with standard input by this way:

```bash
$ sudachiclone
```

`sudachiclone` has 4 subcommands (default: `tokenize`)

```bash
$ sudachiclone -h
Japanese Morphological Analyzer

USAGE:
    sudachiclone [FLAGS] [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -q               Silence all output
    -V, --version    Prints version information
    -v               Increase message verbosity

OPTIONS:
    -z <timestamp>        prepend timestamp to log lines [possible values: none, sec, ms, ns]

SUBCOMMANDS:
    build       Build Sudachi Dictionary
    help        Prints this message or the help of the given subcommand(s)
    link        Link Default Dict Package
    tokenize    Tokenize Text
    ubuild      Build User Dictionary
```

```bash
$ sudachiclone tokenize -h
sudachiclone-tokenize 0.2.1
Tokenize Text

USAGE:
    sudachiclone tokenize [FLAGS] [OPTIONS] [in_files]...

FLAGS:
    -h, --help       (default) see `tokenize -h`
    -a               print all of the fields
    -V, --version    Prints version information

OPTIONS:
    -o <fpath_out>            the output file
    -r <fpath_setting>        the setting file in JSON format
    -m <mode>                 the mode of splitting [possible values: A, B, C]
    -p <python_exe>           path to Python executable

ARGS:
    <in_files>...    text written in utf-8
```

```bash
$ sudachiclone link -h
sudachiclone-link
Link Default Dict Package

USAGE:
    sudachiclone link [OPTIONS]

FLAGS:
    -h, --help       see `link -h`
    -V, --version    Prints version information

OPTIONS:
    -t <dict_type>         dict dict [default: core]  [possible values: small, core, full]
    -p <python_exe>        path to Python executable```

```bash
$ sudachiclone build -h
sudachiclone-build
Build Sudachi Dictionary

USAGE:
    sudachiclone build [FLAGS] [OPTIONS] -m [in_files]

FLAGS:
    -h, --help       see `build -h`
    -m               connection matrix file with MeCab's matrix.def format
    -V, --version    Prints version information

OPTIONS:
    -d <description>        description comment to be embedded on dictionary [default: ]
    -o <out_file>           output file (default: system.dic) [default: system.dic]

ARGS:
    <in_files>    source files with CSV format (one of more)
```

## As a Rust package

Here is an example usage:

```rust
use sudachiclone::prelude::*;

let dictionary = Dictionary::setup(None, None, None).unwrap();
let tokenizer = dictionary.create();

// Multi-granular tokenization
// using `system_core.dic` or `system_full.dic` version 20190781
// you may not be able to replicate this particular example due to dictionary you use

for m in tokenizer.tokenize("国家公務員", Some(SplitMode::C), None).unwrap() {
    println!("{}", m.surface());
};
// => 国家公務員

for m in tokenizer.tokenize("国家公務員", Some(SplitMode::B), None).unwrap() {
    println!("{}", m.surface());
};
// => 国家
// => 公務員

for m in tokenizer.tokenize("国家公務員", Some(SplitMode::A), None).unwrap() {
    println!("{}", m.surface());
};
// => 国家
// => 公務
// => 員

// Morpheme information

let m = tokenizer.tokenize("食べ", Some(SplitMode::A), None).unwrap().get(0).unwrap();
println!("{}", m.surface());
// => 食べ
println!("{}", m.dictionary_form());
// => 食べる
println!("{}", m.reading_form());
// => タベ
println!("{:?}", m.part_of_speech());
// => ["動詞", "一般", "*", "*", "下一段-バ行", "連用形-一般"]

// Normalization

println!("{}", tokenizer.tokenize("附属", Some(SplitMode::A), None).unwrap().get(0).unwrap().normalized_form());
// => 付属

println!("{}", tokenizer.tokenize("SUMMER", Some(SplitMode::A), None).unwrap().get(0).unwrap().normalized_form());
// => サマー

println!("{}", tokenizer.tokenize("シュミレーション", Some(SplitMode::A), None).unwrap().get(0).unwrap().normalized_form());
// => シミュレーション
```

## License

[Apache 2.0](./LICENSE).
