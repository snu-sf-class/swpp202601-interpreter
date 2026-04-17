use nom::{
  branch::alt,
  bytes::complete::{tag, take_while1},
  character::{
    complete::{multispace0, newline},
    is_alphanumeric,
  },
  multi::many0,
  sequence::{preceded, terminated},
  IResult,
};

use crate::{
  parser::{comment_empty_parser, inst::inst_parser, line_num_parser},
  program::SwppBlock,
};

use super::common::ParserResult;

pub fn bname_parser(input: &str) -> IResult<&str, &str> {
  preceded(
    tag("."),
    take_while1(|c: char| is_alphanumeric(c as u8) || c == '_' || c == '-' || c == '.'),
  )(input)
}

#[test]
fn bname_test() {
  let bname = ".hello 5 .hi .def";
  println!("{:?}", bname_parser(bname))
}

pub fn block_parser(input: &str) -> IResult<&str, ParserResult> {
  let (rem, _) = multispace0(input)?;
  // println!("1: {:?}", rem);
  let (rem, line_num) = line_num_parser(rem)?;
  let (rem, bname) = terminated(bname_parser, tag(":"))(rem)?;
  // println!("2: {:?}", rem);
  let (rem, _) = multispace0(rem)?;
  // println!("3: {:?}", rem);

  let inst_comment_parser = alt((comment_empty_parser, inst_parser));

  let (rem, inst_vec) = many0(terminated(inst_comment_parser, newline))(rem)?;

  // println!("4: {:?}", rem);

  let inst_vec = inst_vec
    .into_iter()
    .filter_map(|inst| match inst {
      ParserResult::Inst(i) => Some(i),
      ParserResult::Comment => None,
      _ => unreachable!(),
    })
    .collect();

  let block = SwppBlock {
    block_name: bname.to_owned(),
    inst_vec,
    start_loc: line_num,
  };

  Ok((rem, ParserResult::Block(block)))
}
