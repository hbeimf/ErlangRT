use rt_defs::{Word, SWord};
use rt_defs;
use bin_reader::{BinaryReader, ReadError};
use rt_defs::term_builder::{ITermBuilder, IListBuilder, ITupleBuilder};

use num;
use num::ToPrimitive;


/// Errors indicating a problem with External Term Format parser.
#[derive(Debug)]
pub enum ETFError {
  ParseError(String),
  ReadError(ReadError),
}


impl From<ReadError> for ETFError {
  fn from(e: ReadError) -> Self { ETFError::ReadError(e) }
}


pub type Hopefully<T> = Result<T, ETFError>;


#[repr(u8)]
#[allow(dead_code)]
enum Tag {
  ETF = 131,
  NewFloat = 70,
  BitBinary = 77,
  AtomCacheRef_ = 82,
  SmallInteger = 97,
  Integer = 98,
  Float = 99,
  AtomDeprecated = 100,
  // deprecated?
  Reference = 101,
  Port = 102,
  Pid = 103,
  SmallTuple = 104,
  LargeTuple = 105,
  Nil = 106,
  String = 107,
  List = 108,
  Binary = 109,
  SmallBig = 110,
  LargeBig = 111,
  NewFun = 112,
  Export = 113,
  NewReference = 114,
  SmallAtomDeprecated = 115,
  // deprecated?
  Map = 116,
  Fun = 117,
  AtomUtf8 = 118,
  SmallAtomUtf8 = 119,
}


fn module() -> &'static str { "external_term_format: " }


fn fail<TermType: Copy>(msg: String) -> Hopefully<TermType> {
  Err(ETFError::ParseError(msg))
}


/// Given a binary reader `r` parse term and return it, `heap` is used to
/// allocate space for larger boxed terms.
pub fn decode<TermBuilderT: ITermBuilder>(
  r: &mut BinaryReader, tb: &mut TermBuilderT)
  -> Hopefully<TermBuilderT::TermT>
{
  let etf_tag = r.read_u8();
  if etf_tag != Tag::ETF as u8 {
    let msg = format!("{}Expected ETF tag byte 131, got {}", module(), etf_tag);
    return fail(msg);
  }
  decode_naked::<TermBuilderT>(r, tb)
}


/// Given an encoded term without ETF tag (131u8), read the term from `r` and
/// place boxed term parts on heap `heap`.
pub fn decode_naked<TermBuilderT: ITermBuilder>(
  r: &mut BinaryReader, tb: &mut TermBuilderT)
  -> Hopefully<TermBuilderT::TermT>
{
  let term_tag = r.read_u8();
  match term_tag {
    x if x == Tag::List as u8 =>
      decode_list::<TermBuilderT>(r, tb),

    x if x == Tag::String as u8 =>
      decode_string::<TermBuilderT>(r, tb),

    x if x == Tag::AtomDeprecated as u8 =>
      decode_atom_latin1::<TermBuilderT>(r, tb),

    x if x == Tag::SmallInteger as u8 =>
      decode_u8::<TermBuilderT>(r, tb),

    x if x == Tag::Integer as u8 =>
      decode_s32::<TermBuilderT>(r, tb),

    x if x == Tag::Nil as u8 => Ok(tb.create_nil()),

    x if x == Tag::LargeTuple as u8 => {
      let size = r.read_u32be() as Word;
      decode_tuple::<TermBuilderT>(r, size, tb)
    }

    x if x == Tag::SmallTuple as u8 => {
      let size = r.read_u8() as Word;
      decode_tuple::<TermBuilderT>(r, size, tb)
    }

    x if x == Tag::LargeBig as u8 => {
      let size = r.read_u32be() as Word;
      decode_big::<TermBuilderT>(r, size, tb)
    }

    x if x == Tag::SmallBig as u8 => {
      let size = r.read_u8() as Word;
      decode_big::<TermBuilderT>(r, size, tb)
    }

    x if x == Tag::Binary as u8 =>
      decode_binary::<TermBuilderT>(r, tb),

    _ => {
      let msg = format!(
        "Don't know how to decode ETF value tag 0x{:x} ({})",
        term_tag, term_tag
      );
      fail(msg)
    }
  }
}


/// Given `size`, read digits for a bigint.
fn decode_big<TermBuilderT: ITermBuilder>(
  r: &mut BinaryReader, size: Word, tb: &mut TermBuilderT)
  -> Hopefully<TermBuilderT::TermT>
{
  let sign = if r.read_u8() == 0 { num::bigint::Sign::Plus }
      else { num::bigint::Sign::Minus };
  let digits = r.read_bytes(size)?;
  let big = num::BigInt::from_bytes_le(sign, &digits);

  // Assert that the number fits into small
  if big.bits() < rt_defs::WORD_BITS - 4 {
    let b_signed = big.to_isize().unwrap();
    return Ok(tb.create_small_s(b_signed));
  }

  // Determine storage size in words
  Ok(unsafe { tb.create_bignum(big) })
}


fn decode_binary<TermBuilderT: ITermBuilder>(
  r: &mut BinaryReader, tb: &mut TermBuilderT)
  -> Hopefully<TermBuilderT::TermT>
{
  let n_bytes = r.read_u32be() as usize;
  if n_bytes == 0 {
    return Ok(tb.create_empty_binary())
  }

  let data = r.read_bytes(n_bytes)?;
  Ok(unsafe { tb.create_binary(&data) })
}


/// Given arity, allocate a tuple and read its elements sequentially.
fn decode_tuple<TermBuilderT: ITermBuilder>(
  r: &mut BinaryReader, size: Word, tb: &mut TermBuilderT)
  -> Hopefully<TermBuilderT::TermT>
{
  let mut tuple_builder = tb.create_tuple_builder(size);
  for i in 0..size {
    let elem = decode_naked(r, tb)?;
    unsafe { tuple_builder.set_element_base0(i, elem) }
  }
  Ok(tuple_builder.make_term())
}


fn decode_u8<TermBuilderT: ITermBuilder>(
  r: &mut BinaryReader, tb: &mut TermBuilderT)
  -> Hopefully<TermBuilderT::TermT>
{
  let val = r.read_u8();
  Ok(tb.create_small_s(val as SWord))
}


fn decode_s32<TermBuilderT: ITermBuilder>(
  r: &mut BinaryReader, tb: &mut TermBuilderT)
  -> Hopefully<TermBuilderT::TermT>
{
  let val = r.read_u32be() as i32;
  Ok(tb.create_small_s(val as SWord))
}


fn decode_atom_latin1<TermBuilderT: ITermBuilder>(
  r: &mut BinaryReader, tb: &mut TermBuilderT)
  -> Hopefully<TermBuilderT::TermT>
{
  let sz = r.read_u16be();
  let val = r.read_str_latin1(sz as Word).unwrap();
  Ok(tb.create_atom_str(&val))
}


fn decode_list<TermBuilderT: ITermBuilder>(
  r: &mut BinaryReader, tb: &mut TermBuilderT)
  -> Hopefully<TermBuilderT::TermT>
{
  let n_elem = r.read_u32be();
  if n_elem == 0 {
    return Ok(tb.create_nil());
  }

  let mut list_builder = tb.create_list_builder();
  let n_elem_minus_one = n_elem - 1;

  for i in 0..n_elem {
    let another = decode_naked(r, tb)?;
    unsafe { list_builder.set(another) }

    if i < n_elem_minus_one {
      unsafe { list_builder.next() }
    }
  }

  // Decode tail, possibly a nil
  let tl = decode_naked(r, tb)?;
  unsafe { list_builder.end(tl) }
  Ok(list_builder.make_term())
}


/// A string of bytes encoded as tag 107 (String) with 16-bit length.
fn decode_string<TermBuilderT: ITermBuilder>(
  r: &mut BinaryReader, tb: &mut TermBuilderT)
  -> Hopefully<TermBuilderT::TermT>
{
  let n_elem = r.read_u16be();
  if n_elem == 0 {
    return Ok(tb.create_nil());
  }

  // Using mutability build list forward creating many cells and linking them
  let mut list_builder = tb.create_list_builder();
  let n_elem_minus_one = n_elem - 1;

  for i in 0..n_elem {
    let elem = r.read_u8();
    unsafe {
      let another = tb.create_small_s(elem as SWord);
      list_builder.set(another)
    }

    // Keep building forward
    if i < n_elem_minus_one {
      unsafe { list_builder.next() }
    }
  }

  unsafe { list_builder.end(tb.create_nil()) }
  Ok(list_builder.make_term())
}
