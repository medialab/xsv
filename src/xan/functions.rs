use std::borrow::Cow;
use std::cmp::max;
use std::cmp::{Ordering, PartialOrd};
use std::fs::File;
use std::io::Read;
use std::ops::{Add, Mul, Sub};
use std::path::PathBuf;

use encoding::{label::encoding_from_whatwg_label, DecoderTrap};
use flate2::read::GzDecoder;
use unidecode::unidecode;
use uuid::Uuid;

use super::error::{CallError, SpecifiedCallError};
use super::types::{BoundArgument, BoundArguments, DynamicNumber, DynamicValue};

type FunctionResult = Result<DynamicValue, CallError>;

// TODO: count should be able to take regex
// TODO: deal with list in sequence_compare & contains
// TODO: in list, empty, not empty
// TODO: division must take integer vs. float into account
// TODO: replace
// TODO: we could also have ranges of columns and vec map etc.
// TODO: random, stats etc.
pub fn call<'a>(name: &str, args: BoundArguments) -> Result<BoundArgument<'a>, SpecifiedCallError> {
    Ok(match name {
        "abs" => abs(args),
        "abspath" => abspath(args),
        "add" => arithmetic_op(args, Add::add),
        "and" => and(args),
        "coalesce" => coalesce(args),
        "concat" => concat(args),
        "contains" => contains(args),
        "count" => count(args),
        "eq" => number_compare(args, Ordering::is_eq),
        "endswith" => endswith(args),
        "err" => err(args),
        "first" => first(args),
        "get" => get(args),
        "gt" => number_compare(args, Ordering::is_gt),
        "gte" => number_compare(args, Ordering::is_ge),
        "join" => join(args),
        "last" => last(args),
        "len" => len(args),
        "lt" => number_compare(args, Ordering::is_lt),
        "lte" => number_compare(args, Ordering::is_le),
        "ltrim" => ltrim(args),
        "lower" => lower(args),
        "match" => is_match(args),
        "mul" => arithmetic_op(args, Mul::mul),
        "neq" => number_compare(args, Ordering::is_ne),
        "not" => not(args),
        "or" => or(args),
        "pathjoin" => pathjoin(args),
        "read" => read(args),
        "rtrim" => rtrim(args),
        "slice" => slice(args),
        "split" => split(args),
        "startswith" => startswith(args),
        "sub" => arithmetic_op(args, Sub::sub),
        "s_eq" => sequence_compare(args, Ordering::is_eq),
        "s_gt" => sequence_compare(args, Ordering::is_gt),
        "s_gte" => sequence_compare(args, Ordering::is_ge),
        "s_lt" => sequence_compare(args, Ordering::is_lt),
        "s_lte" => sequence_compare(args, Ordering::is_le),
        "s_neq" => sequence_compare(args, Ordering::is_ne),
        "trim" => trim(args),
        "typeof" => type_of(args),
        "unidecode" => apply_unidecode(args),
        "upper" => upper(args),
        "uuid" => uuid(args),
        "val" => val(args),
        _ => Err(CallError::UnknownFunction(name.to_string())),
    })
    .and_then(|result| match result {
        Ok(value) => Ok(Cow::Owned(value)),
        Err(err) => Err(SpecifiedCallError {
            function_name: name.to_string(),
            reason: err,
        }),
    })
}

// Strings
fn trim(args: BoundArguments) -> FunctionResult {
    args.validate_min_max_arity(1, 2)?;

    let string = args.get(0).unwrap().try_as_str()?;
    let arg2 = args.get(1);

    Ok(match arg2 {
        None => DynamicValue::from(string.trim()),
        Some(arg) => {
            let pattern = arg.try_as_str()?.chars().collect::<Vec<char>>();
            DynamicValue::from(string.trim_matches(|c| pattern.contains(&c)))
        }
    })
}

fn ltrim(args: BoundArguments) -> FunctionResult {
    args.validate_min_max_arity(1, 2)?;

    let string = args.get(0).unwrap().try_as_str()?;
    let arg2 = args.get(1);

    Ok(match arg2 {
        None => DynamicValue::from(string.trim_start()),
        Some(arg) => {
            let pattern = arg.try_as_str()?.chars().collect::<Vec<char>>();
            DynamicValue::from(string.trim_start_matches(|c| pattern.contains(&c)))
        }
    })
}

fn rtrim(args: BoundArguments) -> FunctionResult {
    args.validate_min_max_arity(1, 2)?;

    let string = args.get(0).unwrap().try_as_str()?;
    let arg2 = args.get(1);

    Ok(match arg2 {
        None => DynamicValue::from(string.trim_end()),
        Some(arg) => {
            let pattern = arg.try_as_str()?.chars().collect::<Vec<char>>();
            DynamicValue::from(string.trim_end_matches(|c| pattern.contains(&c)))
        }
    })
}

fn split(args: BoundArguments) -> FunctionResult {
    args.validate_min_max_arity(2, 3)?;
    let args = args.getn_opt(3);

    let to_split = args[0].unwrap().try_as_str()?;
    let pattern = args[1].unwrap().try_as_str()?;
    let count = args[2];

    let splitted: Vec<DynamicValue> = if let Some(c) = count {
        to_split
            .splitn(c.try_as_usize()? + 1, &*pattern)
            .map(DynamicValue::from)
            .collect()
    } else {
        to_split.split(&*pattern).map(DynamicValue::from).collect()
    };

    Ok(DynamicValue::from(splitted))
}

fn lower(args: BoundArguments) -> FunctionResult {
    Ok(DynamicValue::from(args.get1_as_str()?.to_lowercase()))
}

fn upper(args: BoundArguments) -> FunctionResult {
    Ok(DynamicValue::from(args.get1_as_str()?.to_uppercase()))
}

fn len(args: BoundArguments) -> FunctionResult {
    let arg = args.get1()?;

    Ok(DynamicValue::from(match arg.as_ref() {
        DynamicValue::List(list) => list.len(),
        _ => arg.try_as_str()?.len(),
    }))
}

fn count(args: BoundArguments) -> FunctionResult {
    let (string, pattern) = args.get2_as_str()?;

    Ok(DynamicValue::from(string.matches(pattern.as_ref()).count()))
}

fn startswith(args: BoundArguments) -> FunctionResult {
    let (string, pattern) = args.get2_as_str()?;

    Ok(DynamicValue::from(string.starts_with(&*pattern)))
}

fn endswith(args: BoundArguments) -> FunctionResult {
    let (string, pattern) = args.get2_as_str()?;

    Ok(DynamicValue::from(string.ends_with(&*pattern)))
}

fn is_match(args: BoundArguments) -> FunctionResult {
    let (arg1, arg2) = args.get2()?;
    let string = arg1.try_as_str()?;
    let regex = arg2.try_as_regex()?;

    Ok(DynamicValue::from(regex.is_match(&string)))
}

fn concat(args: BoundArguments) -> FunctionResult {
    args.validate_min_arity(1)?;

    let mut args_iter = args.into_iter();
    let first = args_iter.next().unwrap();

    match first.as_ref() {
        DynamicValue::List(list) => {
            let mut result: Vec<DynamicValue> = list.clone();

            for arg in args_iter {
                result.push(arg.as_ref().clone());
            }

            Ok(DynamicValue::List(result))
        }
        value => {
            let mut result = String::new();
            result.push_str(&value.try_as_str()?);

            for arg in args_iter {
                result.push_str(&arg.try_as_str()?);
            }

            Ok(DynamicValue::from(result))
        }
    }
}

fn apply_unidecode(args: BoundArguments) -> FunctionResult {
    let arg = args.get1_as_str()?;

    Ok(DynamicValue::from(unidecode(&arg)))
}

// Lists & Sequences
fn first(args: BoundArguments) -> FunctionResult {
    let arg = args.get1()?;

    Ok(match arg.as_ref() {
        DynamicValue::String(value) => DynamicValue::from(value.chars().next()),
        DynamicValue::List(list) => match list.first() {
            None => DynamicValue::None,
            Some(value) => value.clone(),
        },
        value => {
            return Err(CallError::Cast((
                value.type_of().to_string(),
                "sequence".to_string(),
            )))
        }
    })
}

fn last(args: BoundArguments) -> FunctionResult {
    let arg = args.get1()?;

    Ok(match arg.as_ref() {
        DynamicValue::String(value) => DynamicValue::from(value.chars().rev().next()),
        DynamicValue::List(list) => match list.last() {
            None => DynamicValue::None,
            Some(value) => value.clone(),
        },
        value => {
            return Err(CallError::Cast((
                value.type_of().to_string(),
                "sequence".to_string(),
            )))
        }
    })
}

fn get(args: BoundArguments) -> FunctionResult {
    let (target, index) = args.get2()?;
    let mut index = index.try_as_i64()?;

    Ok(match target.as_ref() {
        DynamicValue::String(value) => {
            if index < 0 {
                index = value.len() as i64 + index;
            }

            if index < 0 {
                DynamicValue::None
            } else {
                DynamicValue::from(value.chars().nth(index as usize))
            }
        }
        DynamicValue::List(list) => {
            if index < 0 {
                index = list.len() as i64 + index;
            }

            if index < 0 {
                DynamicValue::None
            } else {
                match list.get(index as usize) {
                    None => DynamicValue::None,
                    Some(value) => value.clone(),
                }
            }
        }
        value => {
            return Err(CallError::Cast((
                value.type_of().to_string(),
                "sequence".to_string(),
            )))
        }
    })
}

fn slice(args: BoundArguments) -> FunctionResult {
    args.validate_min_max_arity(2, 3)?;

    let args = args.getn_opt(3);

    let target = args[0].unwrap();

    match target.as_ref() {
        DynamicValue::String(string) => {
            let mut lo = args[1].unwrap().try_as_i64()?;
            let opt_hi = args[2];

            let chars = string.chars();

            let substring: String = match opt_hi {
                None => {
                    if lo < 0 {
                        let l = string.chars().count();
                        lo = max(0, l as i64 + lo);

                        chars.skip(lo as usize).collect()
                    } else {
                        chars.skip(lo as usize).collect()
                    }
                }
                Some(hi_value) => {
                    let mut hi = hi_value.try_as_i64()?;

                    if lo < 0 {
                        "".to_string()
                    } else {
                        if hi < 0 {
                            let l = string.chars().count();
                            hi = max(0, l as i64 + hi);
                        }

                        chars.skip(lo as usize).take((hi - lo) as usize).collect()
                    }
                }
            };

            Ok(DynamicValue::from(substring))
        }
        DynamicValue::List(_) => Err(CallError::NotImplemented("list".to_string())),
        value => {
            return Err(CallError::Cast((
                value.type_of().to_string(),
                "sequence".to_string(),
            )))
        }
    }
}

fn join(args: BoundArguments) -> FunctionResult {
    let (arg1, arg2) = args.get2()?;

    let list = arg1.try_as_list()?;
    let joiner = arg2.try_as_str()?;

    let mut string_list: Vec<Cow<str>> = Vec::new();

    for value in list {
        string_list.push(value.try_as_str()?);
    }

    Ok(DynamicValue::from(string_list.join(&joiner)))
}

fn contains(args: BoundArguments) -> FunctionResult {
    let (arg1, arg2) = args.get2()?;

    match arg1.as_ref() {
        DynamicValue::String(text) => {
            let pattern = arg2.try_as_str()?;

            Ok(DynamicValue::from(text.contains(&*pattern)))
        }
        DynamicValue::List(_) => Err(CallError::NotImplemented("list".to_string())),
        value => {
            return Err(CallError::Cast((
                value.type_of().to_string(),
                "sequence".to_string(),
            )))
        }
    }
}

// Arithmetics
fn arithmetic_op<F>(args: BoundArguments, op: F) -> FunctionResult
where
    F: FnOnce(DynamicNumber, DynamicNumber) -> DynamicNumber,
{
    let (a, b) = args.get2_as_numbers()?;
    Ok(DynamicValue::from(op(a, b)))
}

fn abs(args: BoundArguments) -> FunctionResult {
    Ok(DynamicValue::from(args.get1_as_number()?.abs()))
}

// Utilities
fn coalesce(args: BoundArguments) -> FunctionResult {
    for arg in args {
        if arg.is_truthy() {
            return Ok(arg.into_owned());
        }
    }

    Ok(DynamicValue::None)
}

// Boolean
fn not(args: BoundArguments) -> FunctionResult {
    Ok(DynamicValue::from(!args.get1_as_bool()?))
}

fn and(args: BoundArguments) -> FunctionResult {
    let (a, b) = args.get2_as_bool()?;
    Ok(DynamicValue::from(a && b))
}

fn or(args: BoundArguments) -> FunctionResult {
    let (a, b) = args.get2_as_bool()?;
    Ok(DynamicValue::from(a || b))
}

// Comparison
fn number_compare<F>(args: BoundArguments, validate: F) -> FunctionResult
where
    F: FnOnce(Ordering) -> bool,
{
    let (a, b) = args.get2_as_numbers()?;

    Ok(DynamicValue::from(match a.partial_cmp(&b) {
        Some(ordering) => validate(ordering),
        None => false,
    }))
}

fn sequence_compare<F>(args: BoundArguments, validate: F) -> FunctionResult
where
    F: FnOnce(Ordering) -> bool,
{
    // TODO: deal with lists
    let (a, b) = args.get2_as_str()?;

    Ok(DynamicValue::from(match a.partial_cmp(&b) {
        Some(ordering) => validate(ordering),
        None => false,
    }))
}

// IO
fn abspath(args: BoundArguments) -> FunctionResult {
    let arg = args.get1_as_str()?;
    let mut path = PathBuf::new();
    path.push(arg.as_ref());
    let path = path.canonicalize().unwrap();
    let path = String::from(path.to_str().ok_or(CallError::InvalidPath)?);

    Ok(DynamicValue::from(path))
}

fn pathjoin(args: BoundArguments) -> FunctionResult {
    args.validate_min_arity(2)?;

    let mut path = PathBuf::new();

    for arg in args {
        path.push(arg.try_as_str()?.as_ref());
    }

    let path = String::from(path.to_str().ok_or(CallError::InvalidPath)?);

    Ok(DynamicValue::from(path))
}

fn decoder_trap_from_str(name: &str) -> Result<DecoderTrap, CallError> {
    Ok(match name {
        "strict" => DecoderTrap::Strict,
        "replace" => DecoderTrap::Replace,
        "ignore" => DecoderTrap::Ignore,
        _ => return Err(CallError::UnsupportedDecoderTrap(name.to_string())),
    })
}

fn read(args: BoundArguments) -> FunctionResult {
    args.validate_min_max_arity(1, 3)?;

    let path = args.get(0).unwrap().try_as_str()?;

    // TODO: handle encoding
    let mut file = match File::open(path.as_ref()) {
        Err(_) => return Err(CallError::CannotOpenFile(path.into_owned())),
        Ok(f) => f,
    };

    let contents = match args.get(1) {
        Some(encoding_value) => {
            let encoding_name = encoding_value.try_as_str()?.replace("_", "-");
            let encoding = encoding_from_whatwg_label(&encoding_name);
            let encoding = encoding
                .ok_or_else(|| CallError::UnsupportedEncoding(encoding_name.to_string()))?;

            let decoder_trap = match args.get(2) {
                Some(trap) => decoder_trap_from_str(&trap.try_as_str()?)?,
                None => DecoderTrap::Replace,
            };

            let mut buffer: Vec<u8> = Vec::new();

            if path.ends_with(".gz") {
                let mut gz = GzDecoder::new(file);
                gz.read_to_end(&mut buffer)
                    .map_err(|_| CallError::CannotReadFile(path.into_owned()))?;
            } else {
                file.read_to_end(&mut buffer)
                    .map_err(|_| CallError::CannotReadFile(path.into_owned()))?;
            }

            encoding
                .decode(&buffer, decoder_trap)
                .map_err(|_| CallError::DecodeError)?
        }
        None => {
            let mut buffer = String::new();

            if path.ends_with(".gz") {
                let mut gz = GzDecoder::new(file);
                gz.read_to_string(&mut buffer)
                    .map_err(|_| CallError::CannotReadFile(path.into_owned()))?;
            } else {
                file.read_to_string(&mut buffer)
                    .map_err(|_| CallError::CannotReadFile(path.into_owned()))?;
            }

            buffer
        }
    };

    Ok(DynamicValue::from(contents))
}

// Introspection
fn type_of(args: BoundArguments) -> FunctionResult {
    Ok(DynamicValue::from(args.get1()?.type_of()))
}

// Random
fn uuid(args: BoundArguments) -> FunctionResult {
    args.validate_arity(0)?;

    let id = Uuid::new_v4()
        .to_hyphenated()
        .encode_lower(&mut Uuid::encode_buffer())
        .to_string();

    Ok(DynamicValue::from(id))
}

// Utils
fn err(args: BoundArguments) -> FunctionResult {
    let arg = args.get1_as_str()?;

    Err(CallError::Custom(arg.to_string()))
}

fn val(args: BoundArguments) -> FunctionResult {
    let arg = args.get1()?;

    Ok(arg.as_ref().clone())
}
