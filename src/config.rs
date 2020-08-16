use std::convert::Infallible;
use std::fs::{symlink_metadata, File};
use std::io::{BufReader, Error as IOError, ErrorKind as IOErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::str::FromStr;
use std::string::FromUtf8Error;

use serde_json::{error::Error as SerdeError, Value};
#[cfg(any(target_os = "redox", unix, windows))]
use symlink::{remove_symlink_dir, symlink_dir};
use thiserror::Error;

const SUDACHIDICT_PKG_NAME: &str = "sudachidict";
const SUDACHIDICT_CORE_PKG_NAME: &str = "sudachidict_core";
const SUDACHIDICT_FULL_PKG_NAME: &str = "sudachidict_full";
const SUDACHIDICT_SMALL_PKG_NAME: &str = "sudachidict_small";

#[cfg(not(any(target_os = "redox", unix, windows)))]
fn remove_symlink_dir<P: AsRef<Path>>(_path: P) -> Result<(), IOError> {
  Err(IOError::new(
    IOErrorKind::Other,
    "can't call remove_symlink_dir",
  ))
}
#[cfg(not(any(target_os = "redox", unix, windows)))]
fn symlink_dir<P: AsRef<Path>, Q: AsRef<Path>>(_src: P, _dst: Q) -> Result<(), IOError> {
  Err(IOError::new(IOErrorKind::Other, "can't call symlink_dir"))
}

use super::resources;

#[derive(Error, Debug)]
pub enum ConfigErr {
  #[error("{0}")]
  IOError(#[from] IOError),
  #[error("{0}")]
  SerdeError(#[from] SerdeError),
  #[error("{0}")]
  Infallible(#[from] Infallible),
  #[error("{0}")]
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
      PathBuf::from_str(&here)?
        .parent()
        .map(|p| p.to_path_buf())
        .or_else(|| match PathBuf::from_str(file!()) {
          Ok(p) => p.parent().map(|p| p.to_path_buf()),
          Err(_) => None,
        }),
      "NotFoundParentDir",
    )?;
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
      let path = self.resource_dir.join(p);
      if path.exists() {
        return Ok(path);
      }
    }
    let dict_path = get_sudachi_dict_path()?;
    self.settings.as_object_mut().unwrap().insert(
      String::from("systemDict"),
      Value::String(dict_path.to_str().unwrap().to_string()),
    );
    Ok(dict_path)
  }

  pub fn char_def_path(&self) -> Result<PathBuf, ConfigErr> {
    if let Some(Value::String(p)) = self.settings.get("characterDefinitionFile") {
      let path = self.resource_dir.join(p);
      if path.exists() {
        return Ok(path);
      }
    }
    Err(ConfigErr::CharDefiFileNotFoundError)
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
  #[error("{0}")]
  Infallible(#[from] Infallible),
  #[error("{0}")]
  FromUtf8Error(#[from] FromUtf8Error),
  #[error("{0}")]
  IOError(#[from] IOError),
  #[error("`systemDict` must be specified if `SudachiDict_core` not installed")]
  NotFoundSudachiDictCoreErr,
  #[error("Multiple packages of `SudachiDict_*` installed. Set default dict with link command.")]
  SetDefaultDictErr,
  #[error("unlink faild (dictionary exists)")]
  UnlinkFaildErr,
}

/// Spawn child process that will try to print the path of a Python module
fn get_python_package_path_cmd(pkg_name: &str) -> Result<Child, IOError> {
  // todo(tmfink): make compatible with python 2 and 3
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
  match get_python_package_path_cmd(pkg_name) {
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
  if let Some(dst_path) = get_python_package_path_cmd(SUDACHIDICT_PKG_NAME)?
    .wait_with_output()
    .map(|o| String::from_utf8(o.stdout).ok())
    .ok()
    .and_then(|x| x)
  {
    let dst_path = dst_path.trim();
    if !dst_path.is_empty() {
      if symlink_metadata(&dst_path)?.file_type().is_symlink() {
        remove_symlink_dir(&dst_path)?;
      }
      return if Path::new(&dst_path).exists() {
        Err(SudachiDictErr::UnlinkFaildErr)
      } else {
        Ok(())
      };
    }
  }
  Ok(())
}

fn set_default_dict_package(dict_pkg_name: &str) -> Result<String, SudachiDictErr> {
  unlink_default_dict_package()?;
  let src_path = String::from_utf8(
    get_python_package_path_cmd(dict_pkg_name)?
      .wait_with_output()?
      .stdout,
  )?;
  let src_path = src_path.trim();
  let dst_path = ok_or_io_err(PathBuf::from_str(&src_path)?.parent(), "NotFoundParentDir")?
    .join(SUDACHIDICT_PKG_NAME);
  symlink_dir(&src_path, &dst_path)?;
  Ok(dst_path.to_str().unwrap().to_string())
}

fn get_sudachi_py_package_path() -> Result<String, SudachiDictErr> {
  let output = get_python_package_path_cmd(SUDACHIDICT_PKG_NAME)?.wait_with_output()?;
  if output.status.success() {
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
  } else {
    Err(SudachiDictErr::NotFoundSudachiDictCoreErr)
  }
}

fn create_default_link_for_sudachidict_core() -> Result<(), SudachiDictErr> {
  get_sudachi_dict_path()?;
  if !success_import(SUDACHIDICT_CORE_PKG_NAME) {
    return Err(SudachiDictErr::NotFoundSudachiDictCoreErr);
  }
  if success_import(SUDACHIDICT_FULL_PKG_NAME) {
    return Err(SudachiDictErr::SetDefaultDictErr);
  }
  if success_import(SUDACHIDICT_SMALL_PKG_NAME) {
    return Err(SudachiDictErr::SetDefaultDictErr);
  }

  set_default_dict_package(SUDACHIDICT_CORE_PKG_NAME);
  Ok(())
}

fn get_sudachi_dict_path() -> Result<PathBuf, SudachiDictErr> {
  let package_path = get_sudachi_py_package_path()?;
  Ok(PathBuf::from_str(&package_path)?.join("resources/system.dic"))
}
