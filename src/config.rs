use std::convert::Infallible;
use std::fs::{symlink_metadata, File};
use std::io::{BufReader, Error as IOError, ErrorKind as IOErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::str::FromStr;
use std::string::FromUtf8Error;

use serde_json::{error::Error as SerdeError, Value};
use symlink::{remove_symlink_dir, symlink_dir};
use thiserror::Error;

use super::resources;

#[derive(Error, Debug)]
pub enum ConfigErr {
  #[error("{self:?}")]
  IOError(#[from] IOError),
  #[error("{self:?}")]
  SerdeError(#[from] SerdeError),
  #[error("{self:?}")]
  Infallible(#[from] Infallible),
  #[error("{self:?}")]
  FromUtf8Error(#[from] FromUtf8Error),
  #[error("`characterDefinitionFile` not defined in setting file")]
  CharDefiFileNotFoundError,
}

#[derive(Debug)]
#[allow(non_snake_case)]
pub struct Config {
  pub settings: Value,
  pub DEFAULT_RESOURCEDIR: PathBuf,
  pub DEFAULT_SETTINGFILE: PathBuf,
  pub resource_dir: PathBuf,
}

impl Config {
  pub fn empty() -> Result<Config, ConfigErr> {
    let here = String::from_utf8(Command::new("which").arg("sudachiclone").output()?.stdout)?;
    let dir = ok_or_io_err(
      PathBuf::from_str(&here)?.parent().as_ref(),
      "NotFoundParentDir",
    )?
    .to_path_buf();
    Ok(Config {
      settings: Value::Null,
      DEFAULT_RESOURCEDIR: dir.join("resources"),
      DEFAULT_SETTINGFILE: dir.join("resources/sudachi.json"),
      resource_dir: dir.join("resources"),
    })
  }
  pub fn setup(path: Option<&str>, resource_dir: Option<&str>) -> Result<Config, ConfigErr> {
    let mut config = Config::empty()?;
    let default_setting_file = config.DEFAULT_SETTINGFILE.to_path_buf();
    if path.is_none() {
      resources::write_sudachi_json(&default_setting_file)?;
    }
    let path = ok_or_io_err(
      path.or_else(|| default_setting_file.to_str()),
      "InvalidDefaultSettingFilePath",
    )?;
    let resource_dir = if let Some(d) = resource_dir {
      PathBuf::from_str(d)?
    } else {
      let resource_dir =
        ok_or_io_err(Path::new(path).parent(), "InvalidDefaultSettingDirPath")?.to_path_buf();
      resources::write_resources(&resource_dir)?;
      resource_dir
    };
    let mut buf = String::new();
    BufReader::new(&mut File::open(path)?).read_to_string(&mut buf)?;
    let settings = serde_json::from_str(&buf)?;
    config.resource_dir = resource_dir;
    config.settings = settings;
    Ok(config)
  }
  pub fn system_dict_path(&mut self) -> Result<PathBuf, SudachiDictErr> {
    if let Some(Value::String(p)) = self.settings.get("systemDict") {
      Ok(self.resource_dir.join(p))
    } else {
      let dict_path = create_default_link_for_sudachidict_core()?;
      self.settings.as_object_mut().unwrap().insert(
        String::from("systemDict"),
        Value::String(dict_path.to_str().unwrap().to_string()),
      );
      Ok(dict_path)
    }
  }
  pub fn char_def_path(&self) -> Result<PathBuf, ConfigErr> {
    if let Some(Value::String(p)) = self.settings.get("characterDefinitionFile") {
      Ok(self.resource_dir.join(p))
    } else {
      Err(ConfigErr::CharDefiFileNotFoundError)
    }
  }
  pub fn user_dict_paths(&self) -> Vec<PathBuf> {
    let mut paths = vec![];
    if let Some(Value::Array(arr)) = self.settings.get("userDict") {
      for v in arr {
        if let Value::String(path) = v {
          paths.push(self.resource_dir.join(path));
        }
      }
    }
    paths
  }
}

fn ok_or_io_err<T>(t: Option<T>, err: &str) -> Result<T, IOError> {
  t.ok_or_else(|| IOError::new(IOErrorKind::Other, err))
}

#[derive(Error, Debug)]
pub enum SudachiDictErr {
  #[error("{self:?}")]
  Infallible(#[from] Infallible),
  #[error("{self:?}")]
  FromUtf8Error(#[from] FromUtf8Error),
  #[error("{self:?}")]
  IOError(#[from] IOError),
  #[error("`systemDict` must be specified if `SudachiDict_core` not installed")]
  NotFoundSudachiDictCoreErr,
  #[error("Multiple packages of `SudachiDict_*` installed. Set default dict with link command.")]
  SetDefaultDictErr,
  #[error("unlink faild (dictionary exists)")]
  UnlinkFaildErr,
}

fn get_pip_pkg_path_cmd(pkg_name: &str) -> Result<Child, IOError> {
  let cmd = format!(
    r#"
from importlib import import_module
from pathlib import Path
print(Path(import_module("{}").__file__).parent)
exit()
"#,
    pkg_name
  );
  let mut child = Command::new("python")
    .stdin(Stdio::piped())
    .stderr(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()?;
  child.stdin.as_mut().unwrap().write_all(cmd.as_bytes())?;
  Ok(child)
}

fn success_import(pkg_name: &str) -> bool {
  match get_pip_pkg_path_cmd(pkg_name) {
    Ok(cmd) => {
      let output = cmd.wait_with_output();
      match output {
        Ok(o) => o.status.success(),
        Err(_) => false,
      }
    }
    Err(_) => false,
  }
}

fn unlink_default_dict_package() -> Result<(), SudachiDictErr> {
  if let Some(dst_path) = get_pip_pkg_path_cmd("sudachidict")?
    .wait_with_output()
    .map(|o| String::from_utf8(o.stdout).ok())
    .ok()
    .and_then(|x| x)
  {
    let dst_path = dst_path.trim();
    if symlink_metadata(&dst_path)?.file_type().is_symlink() {
      println!("unlinkng sudachidict");
      remove_symlink_dir(&dst_path)?;
      println!("sudachidict unlinked");
    }
    if Path::new(&dst_path).exists() {
      Err(SudachiDictErr::UnlinkFaildErr)
    } else {
      Ok(())
    }
  } else {
    println!("sudachidict not exists");
    Ok(())
  }
}

fn set_default_dict_package(dict_pkg_name: &str) -> Result<String, SudachiDictErr> {
  unlink_default_dict_package()?;
  let src_path = String::from_utf8(
    get_pip_pkg_path_cmd(dict_pkg_name)?
      .wait_with_output()?
      .stdout,
  )?;
  let src_path = src_path.trim();
  let dst_path =
    ok_or_io_err(PathBuf::from_str(&src_path)?.parent(), "NotFoundParentDir")?.join("sudachidict");
  symlink_dir(&src_path, &dst_path)?;
  Ok(dst_path.to_str().unwrap().to_string())
}

fn get_dict_path() -> Result<String, SudachiDictErr> {
  if let Ok(output) = get_pip_pkg_path_cmd("sudachidict")?.wait_with_output() {
    if output.status.success() {
      return Ok(String::from_utf8(output.stdout)?.trim().to_string());
    }
  }
  if !success_import("sudachidict_core") {
    return Err(SudachiDictErr::NotFoundSudachiDictCoreErr);
  }
  if success_import("sudachidict_full") {
    return Err(SudachiDictErr::SetDefaultDictErr);
  }
  if success_import("sudachidict_small") {
    return Err(SudachiDictErr::SetDefaultDictErr);
  }
  set_default_dict_package("sudachidict_core")
}

fn create_default_link_for_sudachidict_core() -> Result<PathBuf, SudachiDictErr> {
  let dict_path = get_dict_path()?;
  Ok(PathBuf::from_str(&dict_path)?.join("resources/system.dic"))
}
