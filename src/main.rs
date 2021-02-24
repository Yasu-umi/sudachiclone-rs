use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::str::FromStr;

use clap::{crate_name, crate_version, App, Arg, ArgMatches, SubCommand};
use log::info;

use sudachiclone::config::{create_default_link_for_sudachidict_core, Config};
use sudachiclone::dictionary::Dictionary;
use sudachiclone::dictionary_lib::binary_dictionary::BinaryDictionary;
use sudachiclone::dictionary_lib::dictionary_builder::DictionaryBuilder;
use sudachiclone::dictionary_lib::dictionary_header::DictionaryHeader;
use sudachiclone::dictionary_lib::system_dictionary_version::{
  SYSTEM_DICT_VERSION_1, USER_DICT_VERSION_2,
};
use sudachiclone::dictionary_lib::user_dictionary_builder::UserDictionaryBuilder;
use sudachiclone::tokenizer::{CanTokenize, SplitMode, Tokenizer};

// Subcommand names
const TOKENIZE_SUB_CMD: &str = "tokenize";
const LINK_SUB_CMD: &str = "link";
const BUILD_SUB_CMD: &str = "build";
const UBUILD_SUB_CMD: &str = "ubuild";

// Argument names
const DESCRIPTION_ARG: &str = "description";
const DICT_TYPE_ARG: &str = "dict_type";
const FPATH_OUT_ARG: &str = "fpath_out";
const FPATH_SETTING_ARG: &str = "fpath_setting";
const IN_FILES_ARG: &str = "in_files";
const LOG_TIMESTAMP_ARG: &str = "timestamp";
const MATRIX_FILE_ARG: &str = "matrix_file";
const MODE_ARG: &str = "mode";
const OUT_FILE_ARG: &str = "out_file";
const PYTHON_BIN_ARG: &str = "python_exe";
const QUIET_ARG: &str = "quiet";
const PRINT_ALL_ARG: &str = "print_all";
const SYSTEM_DIC_ARG: &str = "system_dic";
const VERBOSE_ARG: &str = "verbose";

fn unwrap<T, E: Error>(t: Result<T, E>) -> T {
  match t {
    Ok(t) => t,
    Err(e) => {
      eprintln!("{}", e);
      exit(1);
    }
  }
}

fn tokenize_loop<R: BufRead, W: Write>(
  read_handle: &mut R,
  write_handle: &mut W,
  tokenizer: Tokenizer,
  mode: Option<SplitMode>,
  print_all: bool,
) {
  let mut input = String::new();

  while let Ok(bytes_read) = read_handle.read_line(&mut input) {
    if bytes_read == 0 {
      break;
    }
    for line in input.trim().split('\n') {
      if let Some(morpheme_list) = tokenizer.tokenize(line, mode, None) {
        for morpheme in morpheme_list {
          let _ = writeln!(write_handle, "{}", morpheme.to_string(print_all).join("\t"));
        }
      }
    }
    let _ = writeln!(write_handle, "EOS");
  }
}

fn tokenize(args: &ArgMatches) {
  let mode = match args.value_of(MODE_ARG) {
    Some("A") => Some(SplitMode::A),
    Some("B") => Some(SplitMode::B),
    Some("C") => Some(SplitMode::C),
    _ => None,
  };

  let fpath_setting = args.value_of(FPATH_SETTING_ARG);
  let python_exe = args.value_of_os(PYTHON_BIN_ARG);
  let print_all = args.is_present(PRINT_ALL_ARG);
  let fpath_out = args.value_of(FPATH_OUT_ARG);

  let dictionary = unwrap(Dictionary::setup(fpath_setting, None, python_exe));
  let tokenizer = dictionary.create();

  let stdin = std::io::stdin();
  let mut read_handle = stdin.lock();

  if let Some(fpath_out) = fpath_out {
    let out_file = OpenOptions::new()
      .create(true)
      .write(true)
      .truncate(true)
      .open(fpath_out);
    let mut out_file = unwrap(out_file);
    tokenize_loop(&mut read_handle, &mut out_file, tokenizer, mode, print_all);
  } else {
    let stdout = std::io::stdout();
    let mut write_handle = stdout.lock();
    tokenize_loop(
      &mut read_handle,
      &mut write_handle,
      tokenizer,
      mode,
      print_all,
    );
  }
}

fn link(args: &ArgMatches) {
  let python_exe = args.value_of_os(PYTHON_BIN_ARG);
  unwrap(create_default_link_for_sudachidict_core(python_exe));
}

fn build(args: &ArgMatches) {
  let description = args.value_of(DESCRIPTION_ARG).unwrap().to_string();
  let header = DictionaryHeader::new(
    SYSTEM_DICT_VERSION_1,
    DictionaryHeader::get_time(),
    description,
  );
  let mut writer = BufWriter::new(unwrap(File::create(args.value_of(OUT_FILE_ARG).unwrap())));
  unwrap(writer.write_all(&unwrap(header.to_bytes())));
  let mut builder = DictionaryBuilder::default();
  let mut matrix_reader =
    BufReader::new(unwrap(File::open(args.value_of(MATRIX_FILE_ARG).unwrap())));
  let lexicon_paths: Vec<&str> = args.values_of(IN_FILES_ARG).unwrap().collect();
  unwrap(builder.build(&lexicon_paths, Some(&mut matrix_reader), &mut writer));
}

fn ubuild(args: &ArgMatches) {
  let system_dic = if let Some(system_dic) = args.value_of(SYSTEM_DIC_ARG) {
    PathBuf::from(system_dic)
  } else {
    let python_exe = args.value_of_os(PYTHON_BIN_ARG);
    let mut config = unwrap(Config::setup(None, None));
    unwrap(config.system_dict_path(python_exe))
  };
  if !system_dic.is_file() {
    eprintln!(
      "{}: error: {} doesn't exist",
      crate_name!(),
      system_dic.to_str().unwrap()
    );
    exit(1);
  }
  let description = args.value_of(DESCRIPTION_ARG).unwrap().to_string();
  let header = DictionaryHeader::new(
    USER_DICT_VERSION_2,
    DictionaryHeader::get_time(),
    description,
  );
  let dictionary = unwrap(BinaryDictionary::from_system_dictionary(system_dic));
  let mut writer = BufWriter::new(unwrap(File::create(args.value_of(OUT_FILE_ARG).unwrap())));
  unwrap(writer.write_all(&header.to_bytes().unwrap()));
  let mut builder = UserDictionaryBuilder::new(dictionary.grammar, dictionary.lexicon);
  let lexicon_paths: Vec<&str> = args.values_of(IN_FILES_ARG).unwrap().collect();
  unwrap(builder.build(&lexicon_paths, &mut writer));
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

trait ClapAppExt {
  fn add_python_exe_arg(self) -> Self;
  fn add_log_args(self) -> Self;
}

impl<'a, 'b> ClapAppExt for clap::App<'a, 'b> {
  fn add_python_exe_arg(self) -> Self {
    self.arg(
      Arg::with_name(PYTHON_BIN_ARG)
        .short("p")
        .takes_value(true)
        .help("path to Python executable")
        .long_help(
          "path to Python executable (used to detect sudachi Python module install location)",
        ),
    )
  }

  fn add_log_args(self) -> Self {
    self
      .arg(
        Arg::with_name(VERBOSE_ARG)
          .short("v")
          .multiple(true)
          .help("Increase message verbosity"),
      )
      .arg(
        Arg::with_name(QUIET_ARG)
          .short("q")
          .help("Silence all output"),
      )
      .arg(
        Arg::with_name(LOG_TIMESTAMP_ARG)
          .short("z")
          .help("prepend timestamp to log lines")
          .takes_value(true)
          .possible_values(&["none", "sec", "ms", "ns"]),
      )
  }
}

fn setup_logging(matches: &clap::ArgMatches) {
  use log::*;
  let verbosity = matches.occurrences_of(VERBOSE_ARG) as usize + 1;
  let quiet = matches.is_present(QUIET_ARG);
  let ts = matches
    .value_of(LOG_TIMESTAMP_ARG)
    .map(|v| {
      stderrlog::Timestamp::from_str(v).unwrap_or_else(|_| {
        clap::Error {
          message: "invalid value for 'timestamp'".into(),
          kind: clap::ErrorKind::InvalidValue,
          info: None,
        }
        .exit()
      })
    })
    .unwrap_or(stderrlog::Timestamp::Off);

  stderrlog::new()
    .module(module_path!())
    .quiet(quiet)
    .verbosity(verbosity)
    .timestamp(ts)
    .init()
    .unwrap();
  info!("setup logging to level {} (quiet={})", verbosity, quiet);
}

fn main() {
  let tokenize_subcommand = SubCommand::with_name(TOKENIZE_SUB_CMD)
    .about("Tokenize Text")
    .help_message("(default) see `tokenize -h`")
    .arg(
      Arg::with_name(FPATH_SETTING_ARG)
        .short("r")
        .takes_value(true)
        .help("the setting file in JSON format"),
    )
    .arg(
      Arg::with_name(MODE_ARG)
        .short("m")
        .takes_value(true)
        .possible_values(&["A", "B", "C"])
        .help("the mode of splitting"),
    )
    .arg(
      Arg::with_name(FPATH_OUT_ARG)
        .short("o")
        .takes_value(true)
        .help("the output file"),
    )
    .arg(
      Arg::with_name(PRINT_ALL_ARG)
        .short("a")
        .help("print all of the fields"),
    )
    .arg(
      Arg::with_name(IN_FILES_ARG)
        .takes_value(true)
        .multiple(true)
        .help("text written in utf-8")
        .validator(in_files_validator),
    )
    .version(crate_version!())
    .add_python_exe_arg();

  let link_subcommand = SubCommand::with_name(LINK_SUB_CMD)
    .about("Link Default Dict Package")
    .help_message("see `link -h`")
    .arg(
      Arg::with_name(DICT_TYPE_ARG)
        .short("t")
        .takes_value(true)
        .possible_values(&["small", "core", "full"])
        .default_value("core")
        .help("dict dict"),
    )
    .add_python_exe_arg();

  let build_subcommand = SubCommand::with_name(BUILD_SUB_CMD)
    .about("Build Sudachi Dictionary")
    .help_message("see `build -h`")
    .arg(
      Arg::with_name(OUT_FILE_ARG)
        .short("o")
        .takes_value(true)
        .default_value("system.dic")
        .help("output file (default: system.dic)"),
    )
    .arg(
      Arg::with_name(DESCRIPTION_ARG)
        .short("d")
        .takes_value(true)
        .default_value("")
        .help("description comment to be embedded on dictionary"),
    )
    .arg(
      Arg::with_name(MATRIX_FILE_ARG)
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
      Arg::with_name(IN_FILES_ARG)
        .takes_value(true)
        .help("source files with CSV format (one of more)"),
    );

  let ubuild_subcommand = SubCommand::with_name(UBUILD_SUB_CMD)
    .about("Build User Dictionary")
    .help_message("see `ubuild -h`")
    .arg(
      Arg::with_name(OUT_FILE_ARG)
        .short("o")
        .takes_value(true)
        .default_value("user.dic")
        .help("output file (default: user.dic)"),
    )
    .arg(
      Arg::with_name(DESCRIPTION_ARG)
        .short("d")
        .takes_value(true)
        .default_value("")
        .help("description comment to be embedded on dictionary"),
    )
    .arg(
      Arg::with_name(SYSTEM_DIC_ARG)
        .short("s")
        .takes_value(true)
        .help("system dictionary (default: linked system_dic, see link -h)"),
    )
    .arg(
      Arg::with_name(IN_FILES_ARG)
        .takes_value(true)
        .help("source files with CSV format (one of more)"),
    );

  let mut app = App::new("Japanese Morphological Analyzer")
    .subcommand(tokenize_subcommand)
    .subcommand(link_subcommand)
    .subcommand(build_subcommand)
    .subcommand(ubuild_subcommand)
    .add_log_args();
  let matches = app.clone().get_matches();

  setup_logging(&matches);
  match matches.subcommand() {
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
