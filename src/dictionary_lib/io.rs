use std::io::{BufRead, Result as IOResult, Seek, SeekFrom, Write};

use byteorder::{LittleEndian, WriteBytesExt};

pub trait CurrentPosition {
  fn position(&mut self) -> IOResult<usize>;
}
impl<S: Seek> CurrentPosition for S {
  fn position(&mut self) -> IOResult<usize> {
    Ok(self.seek(SeekFrom::Current(0))? as usize)
  }
}

pub trait LittleEndianWrite {
  fn write_utf16_str(&mut self, d: &str) -> IOResult<()>;
  fn write_i16(&mut self, n: i16) -> IOResult<()>;
  fn write_u64(&mut self, n: u64) -> IOResult<()>;
  fn write_u32(&mut self, n: u32) -> IOResult<()>;
  fn write_u8(&mut self, n: u8) -> IOResult<()>;
}
impl<W: Write> LittleEndianWrite for W {
  fn write_utf16_str(&mut self, d: &str) -> IOResult<()> {
    for b in d.encode_utf16() {
      self.write_u16::<LittleEndian>(b)?;
    }
    Ok(())
  }
  fn write_i16(&mut self, n: i16) -> IOResult<()> {
    self.write_all(&n.to_le_bytes())
  }
  fn write_u64(&mut self, n: u64) -> IOResult<()> {
    self.write_all(&n.to_le_bytes())
  }
  fn write_u32(&mut self, n: u32) -> IOResult<()> {
    self.write_all(&n.to_le_bytes())
  }
  fn write_u8(&mut self, n: u8) -> IOResult<()> {
    self.write_all(&[n])
  }
}

pub trait Pipe {
  fn pipe_all<W: Write>(&mut self, writer: &mut W) -> IOResult<()>;
}
impl<R: BufRead> Pipe for R {
  fn pipe_all<W: Write>(&mut self, writer: &mut W) -> IOResult<()> {
    writer.write_all(self.fill_buf()?)
  }
}
