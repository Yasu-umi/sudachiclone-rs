use std::error::Error;
use std::fs::File;
use std::io::{stdin, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::exit;

use clap::{crate_name, crate_version, App, Arg, ArgMatches, SubCommand};

use sudachiclone::config::{Config, create_default_link_for_sudachidict_core};
use sudachiclone::dictionary::Dictionary;
use sudachiclone::dictionary_lib::binary_dictionary::BinaryDictionary;
use sudachiclone::dictionary_lib::dictionary_builder::DictionaryBuilder;
use sudachiclone::dictionary_lib::dictionary_header::DictionaryHeader;
use sudachiclone::dictionary_lib::system_dictionary_version::{
  SYSTEM_DICT_VERSION, USER_DICT_VERSION_2,
};
use sudachiclone::dictionary_lib::user_dictionary_builder::UserDictionaryBuilder;
use sudachiclone::tokenizer::{CanTokenize, SplitMode};

const TOKENIZE_SUB_CMD: &str = "tokenize";
const LINK_SUB_CMD: &str = "link";
const BUILD_SUB_CMD: &str = "build";
const UBUILD_SUB_CMD: &str = "ubuild";


fn unwrap<T, E: Error>(t: Result<T, E>) -> T {
  match t {
    Ok(t) => t,
    Err(e) => {
      eprintln!("{}", e);
      exit(1);
    }
  }
}

fn tokenize(args: &ArgMatches) {
  if args.is_present("version") {
    print_version();
    return;
  }
  let mode = match args.value_of("mode") {
    Some("A") => Some(SplitMode::A),
    Some("B") => Some(SplitMode::B),
    Some("C") => Some(SplitMode::C),
    _ => None,
  };
  // fpath_out or stdout

  let dictionary = unwrap(Dictionary::setup(args.value_of("fpath_setting"), None));
  let tokenizer = dictionary.create();

  let mut input = String::new();
  let print_all = args.is_present("print_all");
  loop {
    while let Ok(_) = stdin().read_line(&mut input) {
      for line in input.trim().split('\n') {
        if let Some(morpheme_list) = tokenizer.tokenize(line, mode, None) {
          for morpheme in morpheme_list {
            println!("{}", morpheme.to_string(print_all).join("\t"));
          }
        }
      }
      println!("EOS");
    }
  }
}

fn link(_args: &ArgMatches) {
  unwrap(create_default_link_for_sudachidict_core());
}

fn build(args: &ArgMatches) {
  let description = args.value_of("description").unwrap().to_string();
  let header = DictionaryHeader::new(
    SYSTEM_DICT_VERSION,
    DictionaryHeader::get_time(),
    description,
  );
  let mut writer = BufWriter::new(unwrap(File::create(args.value_of("out_file").unwrap())));
  unwrap(writer.write_all(&unwrap(header.to_bytes())));
  let mut builder = DictionaryBuilder::default();
  let mut matrix_reader = BufReader::new(unwrap(File::open(args.value_of("matrix_file").unwrap())));
  let lexicon_paths: Vec<&str> = args.values_of("in_files").unwrap().collect();
  unwrap(builder.build(&lexicon_paths, Some(&mut matrix_reader), &mut writer));
}

fn ubuild(args: &ArgMatches) {
  let system_dic = if let Some(system_dic) = args.value_of("system_dic") {
    PathBuf::from(system_dic)
  } else {
    let mut config = unwrap(Config::setup(None, None));
    unwrap(config.system_dict_path())
  };
  if !system_dic.is_file() {
    eprintln!(
      "{}: error: {} doesn't exist",
      crate_name!(),
      system_dic.to_str().unwrap()
    );
    exit(1);
  }
  let description = args.value_of("description").unwrap().to_string();
  let header = DictionaryHeader::new(
    USER_DICT_VERSION_2,
    DictionaryHeader::get_time(),
    description,
  );
  let dictionary = unwrap(BinaryDictionary::from_system_dictionary(system_dic));
  let mut writer = BufWriter::new(unwrap(File::create(args.value_of("out_file").unwrap())));
  unwrap(writer.write_all(&header.to_bytes().unwrap()));
  let mut builder = UserDictionaryBuilder::new(dictionary.grammar, dictionary.lexicon);
  let lexicon_paths: Vec<&str> = args.values_of("in_files").unwrap().collect();
  unwrap(builder.build(&lexicon_paths, &mut writer));
}

fn print_version() {
  println!("sudachi {}", crate_version!())
}

fn in_files_validator(in_file: String) -> Result<(), String> {
  if Path::new(&in_file).is_file() {
    Ok(())
  } else {
    Err(format!(
      "{}: error: {} doesn't exist",
      crate_name!(),
      in_file
    ))
  }
}

fn main() {
  let tokenize_subcommand = SubCommand::with_name(TOKENIZE_SUB_CMD)
    .about("Tokenize Text")
    .help_message("(default) see `tokenize -h`")
    .arg(
      Arg::with_name("fpath_setting")
        .short("r")
        .takes_value(true)
        .help("the setting file in JSON format"),
    )
    .arg(
      Arg::with_name("mode")
        .short("m")
        .takes_value(true)
        .possible_values(&["A", "B", "C"])
        .help("the mode of splitting"),
    )
    .arg(
      Arg::with_name("fpath_out")
        .short("o")
        .takes_value(true)
        .help("the output file"),
    )
    .arg(
      Arg::with_name("print_all")
        .short("a")
        .help("print all of the fields"),
    )
    .arg(
      Arg::with_name("print_debug")
        .short("d")
        .help("print the debug information"),
    )
    .arg(
      Arg::with_name("version")
        .short("v")
        .help("print sudachipy version"),
    )
    .arg(
      Arg::with_name("in_files")
        .takes_value(true)
        .multiple(true)
        .help("text written in utf-8")
        .validator(in_files_validator),
    );

  let link_subcommand = SubCommand::with_name(LINK_SUB_CMD)
    .about("Link Default Dict Package")
    .help_message("see `link -h`")
    .arg(
      Arg::with_name("dict_type")
        .short("t")
        .takes_value(true)
        .possible_values(&["small", "core", "full"])
        .default_value("core")
        .help("dict dict"),
    );

  let build_subcommand = SubCommand::with_name(BUILD_SUB_CMD)
    .about("Build Sudachi Dictionary")
    .help_message("see `build -h`")
    .arg(
      Arg::with_name("out_file")
        .short("o")
        .takes_value(true)
        .default_value("system.dic")
        .help("output file (default: system.dic)"),
    )
    .arg(
      Arg::with_name("description")
        .short("d")
        .takes_value(true)
        .default_value("")
        .help("description comment to be embedded on dictionary"),
    )
    .arg(
      Arg::with_name("matrix_file")
        .short("m")
        .required(true)
        .help("connection matrix file with MeCab\'s matrix.def format")
        .validator(|matrix_file| {
          if Path::new(&matrix_file).is_file() {
            Ok(())
          } else {
            Err(format!(
              "{}: error: {} doesn't exist",
              crate_name!(),
              matrix_file
            ))
          }
        }),
    )
    .arg(
      Arg::with_name("in_files")
        .takes_value(true)
        .help("source files with CSV format (one of more)"),
    );

  let ubuild_subcommand = SubCommand::with_name(UBUILD_SUB_CMD)
    .about("Build User Dictionary")
    .help_message("see `ubuild -h`")
    .arg(
      Arg::with_name("out_file")
        .short("o")
        .takes_value(true)
        .default_value("user.dic")
        .help("output file (default: user.dic)"),
    )
    .arg(
      Arg::with_name("description")
        .short("d")
        .takes_value(true)
        .default_value("")
        .help("description comment to be embedded on dictionary"),
    )
    .arg(
      Arg::with_name("system_dic")
        .short("s")
        .takes_value(true)
        .help("system dictionary (default: linked system_dic, see link -h)"),
    )
    .arg(
      Arg::with_name("in_files")
        .takes_value(true)
        .help("source files with CSV format (one of more)"),
    );

  // todo(tmfink): add argument to specify python location
  let mut app = App::new("Japanese Morphological Analyzer")
    .subcommand(tokenize_subcommand)
    .subcommand(link_subcommand)
    .subcommand(build_subcommand)
    .subcommand(ubuild_subcommand);
  let matches = app.clone().get_matches();

  match  matches.subcommand() {
    (TOKENIZE_SUB_CMD, Some(tokenize_matches)) => tokenize(tokenize_matches),
    (LINK_SUB_CMD, Some(link_matches)) => link(link_matches),
    (BUILD_SUB_CMD, Some(build_matches)) => build(build_matches),
    (UBUILD_SUB_CMD, Some(ubuild_matches)) => ubuild(ubuild_matches),
    _ => {
      app.print_help().expect("Unable to write help");
      println!();
    }
  }
}
