//! Bounded, explicitly little-endian worker messages. No native values are logged.
use std::io::{self, Read, Write};

/// Version-one protocol identity; a mismatching peer must fail, not guess.
pub const MAGIC: u32 = 0x5357_5031;
/// Maximum frame payload, excluding its four-byte length prefix.
pub const MAX_FRAME: usize = 16384;

/// Owned public-operation arguments; lengths are checked before dispatch.
#[derive(Clone, Default)]
pub struct Request {
    /// Monotonic step identity, echoed by each worker.
    pub id: u32,
    /// Registry operation ID, or a reserved handshake/constant-check ID.
    pub op: u32,
    /// Integer arguments in the operation schema's declared order.
    pub ints: Vec<i32>,
    /// Double arguments, preserving their input bit patterns.
    pub floats: Vec<f64>,
    /// UTF-8 inputs; None and Some(empty) remain distinct pointer semantics.
    pub texts: Vec<Option<Vec<u8>>>,
}

/// Fully owned result fields; native buffers and pointers cannot escape.
#[derive(Clone, Default)]
pub struct Record {
    /// Echoed request step identity.
    pub id: u32,
    /// Echoed operation identity.
    pub op: u32,
    /// Native status/flags or the explicitly reserved Rust error classification.
    pub code: i32,
    /// Exposed integer outputs, separate from the status channel.
    pub ints: Vec<i32>,
    /// Native or public floating outputs in the independent projector's order.
    pub floats: Vec<f64>,
    /// Raw C bytes or Rust UTF-8 bytes; diagnostics are not trimmed.
    pub texts: Vec<Vec<u8>>,
}

impl Request {
    /// Read an integer argument after the dispatcher validates its exact shape.
    pub fn i(&self, index: usize) -> i32 {
        self.ints[index]
    }
    /// Read a double argument without rounding or unit conversion.
    pub fn f(&self, index: usize) -> f64 {
        self.floats[index]
    }
    /// Read a required text; schema validation must reject a missing value.
    pub fn text(&self, index: usize) -> &str {
        self.optional_text(index).unwrap_or("")
    }
    /// Preserve null versus empty text; decode has already validated UTF-8.
    pub fn optional_text(&self, index: usize) -> Option<&str> {
        self.texts
            .get(index)
            .and_then(|s| s.as_deref())
            .map(|s| std::str::from_utf8(s).expect("UTF-8 request"))
    }
    /// Encode bounded manifest inputs; the framing writer checks payload size.
    pub fn encode(&self) -> Vec<u8> {
        let mut b = Vec::new();
        for n in [
            MAGIC,
            self.id,
            self.op,
            self.ints.len() as u32,
            self.floats.len() as u32,
            self.texts.len() as u32,
        ] {
            put(&mut b, n);
        }
        for n in &self.ints {
            put(&mut b, *n as u32);
        }
        for n in &self.floats {
            b.extend(n.to_bits().to_le_bytes());
        }
        for t in &self.texts {
            match t {
                None => put(&mut b, u32::MAX),
                Some(t) => {
                    put(&mut b, t.len() as u32);
                    b.extend(t);
                }
            }
        }
        b
    }
    /// Reject incomplete, oversized, trailing or invalid UTF-8 input frames.
    pub fn decode(b: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(b);
        c.magic()?;
        let id = c.u32()?;
        let op = c.u32()?;
        let ni = c.count(32)?;
        let nf = c.count(64)?;
        let nt = c.count(4)?;
        let ints = (0..ni)
            .map(|_| c.u32().map(|n| n as i32))
            .collect::<io::Result<_>>()?;
        let floats = (0..nf).map(|_| c.f64()).collect::<io::Result<_>>()?;
        let mut texts = Vec::new();
        for _ in 0..nt {
            let n = c.u32()?;
            texts.push(if n == u32::MAX {
                None
            } else {
                if n > 1024 {
                    return Err(invalid("oversized request text"));
                }
                let bytes = c.bytes(n as usize)?;
                std::str::from_utf8(bytes).map_err(|_| invalid("request is not UTF-8"))?;
                Some(bytes.to_vec())
            });
        }
        c.end()?;
        Ok(Self {
            id,
            op,
            ints,
            floats,
            texts,
        })
    }
}

impl Record {
    /// Encode owned fields without decimal formatting or C-struct padding.
    pub fn encode(&self) -> Vec<u8> {
        let mut b = Vec::new();
        for n in [
            MAGIC,
            self.id,
            self.op,
            self.code as u32,
            self.ints.len() as u32,
            self.floats.len() as u32,
            self.texts.len() as u32,
        ] {
            put(&mut b, n);
        }
        for n in &self.ints {
            put(&mut b, *n as u32);
        }
        for n in &self.floats {
            b.extend(n.to_bits().to_le_bytes());
        }
        for t in &self.texts {
            put(&mut b, t.len() as u32);
            b.extend(t);
        }
        b
    }
    /// Decode checked field counts while retaining non-UTF-8 native bytes.
    pub fn decode(b: &[u8]) -> io::Result<Self> {
        let mut c = Cursor::new(b);
        c.magic()?;
        let id = c.u32()?;
        let op = c.u32()?;
        let code = c.u32()? as i32;
        let ni = c.count(512)?;
        let nf = c.count(1024)?;
        let nt = c.count(256)?;
        let ints = (0..ni)
            .map(|_| c.u32().map(|n| n as i32))
            .collect::<io::Result<_>>()?;
        let floats = (0..nf).map(|_| c.f64()).collect::<io::Result<_>>()?;
        let mut texts = Vec::new();
        for _ in 0..nt {
            let n = c.count(4096)?;
            texts.push(c.bytes(n)?.to_vec());
        }
        c.end()?;
        Ok(Self {
            id,
            op,
            code,
            ints,
            floats,
            texts,
        })
    }
}

/// Append an explicitly little-endian integer.
fn put(b: &mut Vec<u8>, n: u32) {
    b.extend(n.to_le_bytes());
}
/// Protocol errors disclose a category, never response payload bytes.
fn invalid(s: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, s)
}
/// Checked reader into one bounded frame; no unchecked offset arithmetic.
struct Cursor<'a> {
    b: &'a [u8],
    offset: usize,
}
impl<'a> Cursor<'a> {
    /// Begin at the first payload byte; the enclosing frame owns the lifetime.
    fn new(b: &'a [u8]) -> Self {
        Self { b, offset: 0 }
    }
    /// Advance only after a checked addition and a successful in-bounds slice.
    fn bytes(&mut self, n: usize) -> io::Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(n)
            .ok_or_else(|| invalid("length overflow"))?;
        let b = self
            .b
            .get(self.offset..end)
            .ok_or_else(|| invalid("truncated frame"))?;
        self.offset = end;
        Ok(b)
    }
    /// Decode exactly four bytes, independent of host endianness.
    fn u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    /// Reconstruct binary64 bits; arithmetic and decimal parsing are forbidden.
    fn f64(&mut self) -> io::Result<f64> {
        Ok(f64::from_bits(u64::from_le_bytes(
            self.bytes(8)?.try_into().unwrap(),
        )))
    }
    /// Apply the schema-independent allocation cap before collecting fields.
    fn count(&mut self, max: usize) -> io::Result<usize> {
        let n = self.u32()? as usize;
        if n > max {
            Err(invalid("excessive count"))
        } else {
            Ok(n)
        }
    }
    /// Fail incompatible protocol revisions rather than interpreting garbage.
    fn magic(&mut self) -> io::Result<()> {
        if self.u32()? == MAGIC {
            Ok(())
        } else {
            Err(invalid("protocol version"))
        }
    }
    /// Every payload byte must be consumed; ignored tails could hide omissions.
    fn end(&self) -> io::Result<()> {
        if self.offset == self.b.len() {
            Ok(())
        } else {
            Err(invalid("trailing bytes"))
        }
    }
}

/// Read a bounded frame. EOF before the prefix is clean; EOF within it is not.
pub fn read_frame(r: &mut impl Read) -> io::Result<Option<Vec<u8>>> {
    let mut len = [0; 4];
    match r.read(&mut len[..1])? {
        0 => return Ok(None),
        1 => (),
        _ => unreachable!(),
    }
    r.read_exact(&mut len[1..])?;
    let n = u32::from_le_bytes(len) as usize;
    if n == 0 || n > MAX_FRAME {
        return Err(invalid("invalid frame length"));
    }
    let mut b = vec![0; n];
    r.read_exact(&mut b)?;
    Ok(Some(b))
}
/// Send one bounded frame and flush so an isolated request cannot stall.
pub fn write_frame(w: &mut impl Write, b: &[u8]) -> io::Result<()> {
    if b.is_empty() || b.len() > MAX_FRAME {
        return Err(invalid("invalid outgoing frame length"));
    }
    w.write_all(&(b.len() as u32).to_le_bytes())?;
    w.write_all(b)?;
    w.flush()
}
