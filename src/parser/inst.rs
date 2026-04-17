use std::collections::HashMap;

use nom::{
  branch::alt,
  bytes::complete::{tag, take_while1},
  character::{
    complete::{alpha1, digit1, space0},
    is_newline,
  },
  multi::{many0, many_till},
  sequence::{preceded, tuple},
  IResult, Parser,
};

use crate::{
  common::{AccessSize, Arg, BitWidth, ICMP},
  inst::{
    InstAssertion, InstAsyncLoad, InstBitwiseAnd, InstBitwiseOr, InstBitwiseXor, InstComparison,
    InstCondBr, InstEAdd, InstESub, InstFunctionCall, InstHeapAllocation, InstHeapFree, InstLoad,
    InstMultiplication, InstOAdd, InstOSub, InstRet, InstShiftLeft, InstShiftRightArithmetic,
    InstShiftRightLogical, InstSignedDivision, InstSignedRemainder, InstStore, InstSwitch,
    InstTernary, InstUncondBr, InstUnsignedDivision, InstUnsignedRemainder, SwppInst, SwppInstKind,
  },
  parser::{line_num_parser, reg::reg_parser},
};

use super::{
  block::bname_parser,
  common::{gen_nom_err, ParserResult, INVALID_BW, INVALID_CONST, INVALID_ICMP, INVALID_SIZE},
  function::fname_parser,
};

pub fn inst_parser(input: &str) -> IResult<&str, ParserResult> {
  let (rem, _) = space0(input)?;
  // println!("5:{:?}",rem);

  let (ret_val, inst) = take_while1(|c| !is_newline(c as u8))(rem)?;
  // println!("5:{:?}",ret_val);
  let control_flow_parser = alt((
    ret_parser,
    ubranch_parser,
    branch_parser,
    switch_parser,
    fcall_parser,
    fcall_assign_parser,
    assertion_parser,
  ));

  let (_, inst) = alt((
    control_flow_parser,
    malloc_parser,
    free_parser,
    load_parser,
    aload_parser,
    store_parser,
    binary_parser,
    icmp_parser,
    ternary_parser,
  ))(inst)?;

  Ok((ret_val, ParserResult::Inst(inst)))
}

fn arg_parser(input: &str) -> IResult<&str, Arg> {
  alt((
    reg_parser.map(Arg::Reg),
    digit1.map(|cst: &str| Arg::Const(cst.parse().unwrap())),
  ))(input)
}

fn ret_parser(input: &str) -> IResult<&str, SwppInst> {
  let (input, line_num) = line_num_parser(input)?;
  let (rem, _) = tag("ret")(input)?;
  // println!("1: {:?}",rem);
  let (rem, _) = space0(rem)?;
  // println!("2: {:?}",rem);
  if rem.is_empty() {
    let inst = SwppInst::new(SwppInstKind::Ret(InstRet::new(None)), line_num.clone());
    Ok((rem, inst))
  } else {
    let (rem, arg) = arg_parser(rem)?;
    let inst = SwppInst::new(SwppInstKind::Ret(InstRet::new(Some(arg))), line_num.clone());
    Ok((rem, inst))
  }
}

fn ubranch_parser(input: &str) -> IResult<&str, SwppInst> {
  let (input, line_num) = line_num_parser(input)?;
  let (rem, _) = tag("br")(input)?;
  let (rem, _) = space0(rem)?;
  let (rem, bname) = bname_parser(rem)?;
  let inst = SwppInst::new(
    SwppInstKind::UBranch(InstUncondBr::new(bname.to_owned())),
    line_num,
  );
  Ok((rem, inst))
}

fn branch_parser(input: &str) -> IResult<&str, SwppInst> {
  let (input, line_num) = line_num_parser(input)?;
  let (rem, _) = tag("br")(input)?;
  let (rem, cond_reg) = preceded(space0, arg_parser)(rem)?;
  let (rem, true_bname) = preceded(space0, bname_parser)(rem)?;
  let (rem, false_bname) = preceded(space0, bname_parser)(rem)?;
  let inst = SwppInst::new(
    SwppInstKind::Branch(InstCondBr::new(
      cond_reg,
      true_bname.to_string(),
      false_bname.to_owned(),
    )),
    line_num,
  );
  Ok((rem, inst))
}

fn switch_parser(input: &str) -> IResult<&str, SwppInst> {
  let (input, line_num) = line_num_parser(input)?;
  let (rem, _) = tag("switch")(input)?;
  let (rem, cond_reg) = preceded(space0, arg_parser)(rem)?;

  let cond_parser = preceded(space0, digit1);
  let block_parser = preceded(space0, bname_parser);

  let case_parser = tuple((cond_parser, block_parser));
  let (rem, (jump_vec, default)) = many_till(case_parser, preceded(space0, bname_parser))(rem)?;

  let jump_vec = jump_vec
    .into_iter()
    .map(|(val, block)| match val.parse() {
      Ok(cst) => Ok((cst, block.to_owned())),
      Err(_) => Err(gen_nom_err(&INVALID_CONST)),
    })
    .collect::<Result<HashMap<u64, String>, _>>()?;

  let inst = InstSwitch::new(cond_reg, jump_vec, default.to_string());
  let inst = SwppInst::new(SwppInstKind::Switch(inst), line_num);
  Ok((rem, inst))
}

fn fcall_parser(input: &str) -> IResult<&str, SwppInst> {
  let (input, line_num) = line_num_parser(input)?;
  let (rem, _) = tag("call")(input)?;
  let (rem, fname) = preceded(space0, fname_parser)(rem)?;
  let (rem, args) = many0(preceded(space0, arg_parser))(rem)?;

  let inst = InstFunctionCall::new(None, fname.to_owned(), args);
  let inst = SwppInst::new(SwppInstKind::FnCall(inst), line_num);
  Ok((rem, inst))
}

fn fcall_assign_parser(input: &str) -> IResult<&str, SwppInst> {
  let (input, line_num) = line_num_parser(input)?;
  let (rem, target) = reg_parser(input)?;
  let (rem, _) = tuple((space0, tag("="), space0))(rem)?;
  let (rem, _) = tag("call")(rem)?;
  let (rem, fname) = preceded(space0, fname_parser)(rem)?;
  let (rem, args) = many0(preceded(space0, arg_parser))(rem)?;

  let inst = InstFunctionCall::new(Some(target), fname.to_owned(), args);
  let inst = SwppInst::new(SwppInstKind::FnCall(inst), line_num);
  Ok((rem, inst))
}

fn assertion_parser(input: &str) -> IResult<&str, SwppInst> {
  let (input, line_num) = line_num_parser(input)?;
  let (rem, _) = tag("assert_eq")(input)?;
  let (rem, lhs) = preceded(space0, arg_parser)(rem)?;
  let (rem, rhs) = preceded(space0, arg_parser)(rem)?;

  let inst = InstAssertion::new(lhs, rhs);
  let inst = SwppInst::new(SwppInstKind::Assert(inst), line_num);
  Ok((rem, inst))
}

fn malloc_parser(input: &str) -> IResult<&str, SwppInst> {
  let (input, line_num) = line_num_parser(input)?;
  let (rem, target) = reg_parser(input)?;
  let (rem, _) = tuple((space0, tag("="), space0))(rem)?;
  let (rem, _) = tag("malloc")(rem)?;
  let (rem, size_reg) = preceded(space0, arg_parser)(rem)?;

  let inst = InstHeapAllocation::new(target, size_reg);
  let inst = SwppInst::new(SwppInstKind::Malloc(inst), line_num);
  Ok((rem, inst))
}

fn free_parser(input: &str) -> IResult<&str, SwppInst> {
  let (input, line_num) = line_num_parser(input)?;
  let (rem, _) = tag("free")(input)?;
  let (rem, addr_reg) = preceded(space0, arg_parser)(rem)?;

  let inst = InstHeapFree::new(addr_reg);
  let inst = SwppInst::new(SwppInstKind::Free(inst), line_num);
  Ok((rem, inst))
}

fn size_parser(input: &str) -> IResult<&str, Option<AccessSize>> {
  let (rem, size) = preceded(space0, digit1)(input)?;
  let size: Result<u64, _> = size.parse();
  if size.is_err() {
    return Ok((rem, None));
  }
  let size = size.unwrap();
  let size = if !(size == 1 || size == 2 || size == 4 || size == 8) {
    None
  } else {
    Some(AccessSize::from(size))
  };

  Ok((rem, size))
}

fn load_parser(input: &str) -> IResult<&str, SwppInst> {
  let (input, line_num) = line_num_parser(input)?;
  let (rem, target) = reg_parser(input)?;
  let (rem, _) = tuple((space0, tag("="), space0))(rem)?;
  let (rem, _) = tag("load")(rem)?;
  let (rem, size) = preceded(space0, size_parser)(rem)?;
  let size = size.ok_or(gen_nom_err(&INVALID_SIZE))?;
  let (rem, addr_reg) = preceded(space0, arg_parser)(rem)?;

  let inst = InstLoad::new(target, addr_reg, size);
  let inst = SwppInst::new(SwppInstKind::Load(inst), line_num);
  Ok((rem, inst))
}

fn aload_parser(input: &str) -> IResult<&str, SwppInst> {
  let (input, line_num) = line_num_parser(input)?;
  let (rem, target) = reg_parser(input)?;
  let (rem, _) = tuple((space0, tag("="), space0))(rem)?;
  let (rem, _) = tag("aload")(rem)?;
  let (rem, size) = preceded(space0, size_parser)(rem)?;
  let size = size.ok_or(gen_nom_err(&INVALID_SIZE))?;
  let (rem, addr_reg) = preceded(space0, arg_parser)(rem)?;

  let inst = InstAsyncLoad::new(target, addr_reg, size);
  let inst = SwppInst::new(SwppInstKind::ALoad(inst), line_num);
  Ok((rem, inst))
}

fn store_parser(input: &str) -> IResult<&str, SwppInst> {
  let (input, line_num) = line_num_parser(input)?;
  let (rem, _) = tag("store")(input)?;
  let (rem, size) = preceded(space0, size_parser)(rem)?;
  let size = size.ok_or(gen_nom_err(&INVALID_SIZE))?;
  let (rem, val_reg) = preceded(space0, arg_parser)(rem)?;

  // println!("1:{:?}",val_reg);
  let (rem, addr_reg) = preceded(space0, arg_parser)(rem)?;

  // println!("2:{:?}",addr_reg);
  let inst = InstStore::new(val_reg, addr_reg, size);
  let inst = SwppInst::new(SwppInstKind::Store(inst), line_num);
  Ok((rem, inst))
}

fn bitwidth_parser(input: &str) -> IResult<&str, Option<BitWidth>> {
  let (rem, bw) = preceded(space0, digit1)(input)?;
  let bw: Result<u64, _> = bw.parse();
  if bw.is_err() {
    return Ok((rem, None));
  }
  let bw = bw.unwrap();
  let bw = if !(bw == 1 || bw == 8 || bw == 16 || bw == 32 || bw == 64) {
    None
  } else {
    Some(BitWidth::from(bw))
  };

  Ok((rem, bw))
}

fn binary_parser(input: &str) -> IResult<&str, SwppInst> {
  let (input, line_num) = line_num_parser(input)?;
  let (rem, target) = reg_parser(input)?;
  let (rem, _) = tuple((space0, tag("="), space0))(rem)?;

  let normal_parser = &mut alt((
    tag("udiv"),
    tag("sdiv"),
    tag("urem"),
    tag("srem"),
    tag("mul"),
    tag("shl"),
    tag("lshr"),
    tag("ashr"),
    tag("and"),
    tag("or"),
    tag("xor"),
    tag("eadd"),
    tag("oadd"),
    tag("esub"),
    tag("osub"),
  ));

  let (rem, op) = normal_parser(rem)?;
  let (rem, lhs) = preceded(space0, arg_parser)(rem)?;
  let (rem, rhs) = preceded(space0, arg_parser)(rem)?;
  let (rem, bw) = preceded(space0, bitwidth_parser)(rem)?;
  let bw = bw.ok_or(gen_nom_err(&INVALID_BW))?;

  let inst = match op {
    "udiv" => {
      let inst = InstUnsignedDivision::new(lhs, rhs, target, bw);
      SwppInst::new(SwppInstKind::UDiv(inst), line_num)
    }
    "sdiv" => {
      let inst = InstSignedDivision::new(lhs, rhs, target, bw);
      SwppInst::new(SwppInstKind::SDiv(inst), line_num)
    }
    "urem" => {
      let inst = InstUnsignedRemainder::new(lhs, rhs, target, bw);
      SwppInst::new(SwppInstKind::URem(inst), line_num)
    }
    "srem" => {
      let inst = InstSignedRemainder::new(lhs, rhs, target, bw);
      SwppInst::new(SwppInstKind::SRem(inst), line_num)
    }
    "mul" => {
      let inst = InstMultiplication::new(lhs, rhs, target, bw);
      SwppInst::new(SwppInstKind::Mul(inst), line_num)
    }
    "shl" => {
      let inst = InstShiftLeft::new(lhs, rhs, target, bw);
      SwppInst::new(SwppInstKind::Shl(inst), line_num)
    }
    "lshr" => {
      let inst = InstShiftRightLogical::new(lhs, rhs, target, bw);
      SwppInst::new(SwppInstKind::Lshr(inst), line_num)
    }
    "ashr" => {
      let inst = InstShiftRightArithmetic::new(lhs, rhs, target, bw);
      SwppInst::new(SwppInstKind::Ashr(inst), line_num)
    }
    "and" => {
      let inst = InstBitwiseAnd::new(lhs, rhs, target, bw);
      SwppInst::new(SwppInstKind::And(inst), line_num)
    }
    "or" => {
      let inst = InstBitwiseOr::new(lhs, rhs, target, bw);
      SwppInst::new(SwppInstKind::Or(inst), line_num)
    }
    "xor" => {
      let inst = InstBitwiseXor::new(lhs, rhs, target, bw);
      SwppInst::new(SwppInstKind::Xor(inst), line_num)
    }
    "eadd" => {
      let inst = InstEAdd::new(lhs, rhs, target, bw);
      SwppInst::new(SwppInstKind::EAdd(inst), line_num)
    }
    "oadd" => {
      let inst = InstOAdd::new(lhs, rhs, target, bw);
      SwppInst::new(SwppInstKind::OAdd(inst), line_num)
    }
    "esub" => {
      let inst = InstESub::new(lhs, rhs, target, bw);
      SwppInst::new(SwppInstKind::ESub(inst), line_num)
    }
    "osub" => {
      let inst = InstOSub::new(lhs, rhs, target, bw);
      SwppInst::new(SwppInstKind::OSub(inst), line_num)
    }
    _ => unreachable!(),
  };

  Ok((rem, inst))
}

fn icmp_parser(input: &str) -> IResult<&str, SwppInst> {
  let (input, line_num) = line_num_parser(input)?;
  let (rem, target) = reg_parser(input)?;
  let (rem, _) = tuple((space0, tag("="), space0))(rem)?;
  let (rem, op) = alt((tag("icmp"), tag("vicmp"), tag("vpicmp")))(rem)?;
  let (rem, cond) = preceded(space0, alpha1)(rem)?;
  let cond = match cond {
    "eq" => ICMP::Eq,
    "ne" => ICMP::Ne,
    "ugt" => ICMP::Ugt,
    "uge" => ICMP::Uge,
    "ult" => ICMP::Ult,
    "ule" => ICMP::Ule,
    "sgt" => ICMP::Sgt,
    "sge" => ICMP::Sge,
    "slt" => ICMP::Slt,
    "sle" => ICMP::Sle,
    _ => panic!("{} : {}", INVALID_ICMP, line_num),
  };
  let (rem, reg1) = preceded(space0, arg_parser)(rem)?;
  let (rem, reg2) = preceded(space0, arg_parser)(rem)?;
  let (rem, bw) = preceded(space0, bitwidth_parser)(rem)?;

  let bw = bw.ok_or(gen_nom_err(INVALID_BW))?;

  let inst = match op {
    "icmp" => {
      let inst = InstComparison::new(reg1, reg2, cond, target, bw);
      SwppInst::new(SwppInstKind::Comp(inst), line_num)
    }
    _ => unreachable!(),
  };
  Ok((rem, inst))
}

fn ternary_parser(input: &str) -> IResult<&str, SwppInst> {
  let (input, line_num) = line_num_parser(input)?;
  let (rem, target) = reg_parser(input)?;
  let (rem, _) = tuple((space0, tag("="), space0))(rem)?;

  let (rem, _) = tag("select")(rem)?;

  let (rem, cond_reg) = preceded(space0, arg_parser)(rem)?;
  let (rem, true_reg) = preceded(space0, arg_parser)(rem)?;
  let (rem, false_reg) = preceded(space0, arg_parser)(rem)?;
  let inst = InstTernary::new(false_reg, true_reg, cond_reg, target);
  let inst = SwppInst::new(SwppInstKind::Select(inst), line_num);

  Ok((rem, inst))
}

#[test]
fn parse_inst_test() {
  let fff = "2:    r1 = malloc r1
    3:    r2 = call read
    4:    r3 = call read
    5:    r1 = mul r3 r4 64
    6:    r5 = sdiv r1 r4 64";
  println!("{:?}", inst_parser(fff));
}
