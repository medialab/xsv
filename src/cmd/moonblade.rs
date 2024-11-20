use std::borrow::Cow;
use std::convert::TryFrom;

use colored::Colorize;
use lazy_static::lazy_static;
use pariter::IteratorExt;
use regex::{Captures, Regex};

use crate::config::{Config, Delimiter};
use crate::moonblade::{DynamicValue, Program, SpecifiedEvaluationError};
use crate::select::SelectColumns;
use crate::util::ImmutableRecordHelpers;
use crate::CliError;
use crate::CliResult;

lazy_static! {
    static ref MAIN_SECTION_REGEX: Regex = Regex::new("(?m)^##{0,2} .+").unwrap();
    static ref FLAG_REGEX: Regex = Regex::new(r"--[\w\-]+").unwrap();
    static ref FUNCTION_REGEX: Regex =
        Regex::new(r"(?i)- ([a-z0-9_]+)\(((?:[a-z0-9=?*_<>]+\s*,?\s*)*)\) -> ([a-z\[\]?| ]+)").unwrap();
    // static ref SPACER_REGEX: Regex = Regex::new(r"(?m)^ {8}([^\n]+)").unwrap();
    static ref UNARY_OPERATOR_REGEX: Regex = Regex::new(r"([!-])x").unwrap();
    static ref BINARY_OPERATOR_REGEX: Regex = Regex::new(
        r"x (==|!=|<[= ]|>[= ]|&& |\|\| |and|or |not in|in|eq|ne|lt|le|gt|ge|//|\*\*|[+\-*/%.]) y"
    )
    .unwrap();
    static ref PIPELINE_OPERATOR_REGEX: Regex = Regex::new(
        r"(trim\(name\) )\|"
    )
    .unwrap();
    static ref SLICE_REGEX: Regex = Regex::new(r"x\[([a-z:]+)\]").unwrap();

    static ref CHEATSHEET_ITEM_REGEX: Regex = Regex::new(r"(?m)^  \. (.+)$").unwrap();
}

fn colorize_functions_help(help: &str) -> String {
    let help = FUNCTION_REGEX.replace_all(help, |caps: &Captures| {
        "- ".to_string()
            + &caps[1].cyan().to_string()
            + &"(".yellow().to_string()
            + &caps[2]
                .split(',')
                .map(|arg| {
                    (if arg == "<expr>" || arg == "<expr>?" {
                        arg.dimmed()
                    } else {
                        arg.red()
                    })
                    .to_string()
                })
                .collect::<Vec<_>>()
                .join(", ")
            + &")".yellow().to_string()
            + " -> "
            + &caps[3].magenta().to_string()
    });

    let help =
        MAIN_SECTION_REGEX.replace_all(&help, |caps: &Captures| caps[0].yellow().to_string());

    // let help = SPACER_REGEX.replace_all(&help, |caps: &Captures| {
    //     " ".repeat(8) + &caps[1].dimmed().to_string()
    // });

    let help = UNARY_OPERATOR_REGEX.replace_all(&help, |caps: &Captures| {
        caps[1].cyan().to_string() + &"x".red().to_string()
    });

    let help = BINARY_OPERATOR_REGEX.replace_all(&help, |caps: &Captures| {
        "x".red().to_string() + " " + &caps[1].cyan().to_string() + " " + &"y".red().to_string()
    });

    let help = PIPELINE_OPERATOR_REGEX.replace_all(&help, |caps: &Captures| {
        caps[1].to_string() + &"|".cyan().to_string()
    });

    let help = SLICE_REGEX.replace_all(&help, |caps: &Captures| {
        "x".red().to_string()
            + "["
            + &caps[1]
                .split(":")
                .map(|part| part.cyan().to_string())
                .collect::<Vec<_>>()
                .join(":")
            + "]"
    });

    let help = FLAG_REGEX.replace_all(&help, |caps: &Captures| caps[0].cyan().to_string());

    help.into_owned()
}

fn colorize_cheatsheet(help: &str) -> String {
    let help = CHEATSHEET_ITEM_REGEX.replace_all(help, |caps: &Captures| {
        "  . ".to_string() + &caps[1].yellow().to_string()
    });

    let help = FLAG_REGEX.replace_all(&help, |caps: &Captures| caps[0].cyan().to_string());

    help.into_owned()
}

pub fn get_moonblade_cheatsheet() -> String {
    let help = "
xan script language cheatsheet (use --functions for comprehensive list of
available functions & operators):

  . Indexing a column by name:
        'name'

  . Indexing column with forbidden characters (e.g. spaces, commas etc.):
        'col(\"Name of film\")'

  . Indexing column by index (0-based):
        'col(2)'

  . Indexing a column by name and 0-based nth (for duplicate headers):
        'col(\"col\", 1)'

  . Indexing a column that may not exist:
        'name?'

  . Applying functions:
        'trim(name)'
        'trim(concat(name, \" \", surname))'

  . Named function arguments:
        'read(path, encoding=\"utf-8\")'

  . Using operators (unary & binary):
        '-nb1'
        'nb1 + nb2'
        '(nb1 > 1) || nb2'

  . Integer literals:
        '1'

  . Float literals:
        '0.5'

  . Boolean literals:
        'true'
        'false'

  . Null literals:
        'null'

  . String literals (can use single or double quotes):
        '\"hello\"'
        \"'hello'\"

  . Regex literals:
        '/john/'
        '/john/i' (case-insensitive)

  . List literals:
        '[1, 2, 3]'
        '[\"one\", \"two\"]

  . Map literals:
        '{one: 1, two: 2}'
        '{leaf: \"hello\", \"nested\": [1, 2, 3]}'

Note that constant expressions will never be evaluated more than once
when parsing the program.

This means that when evaluating the following:
    'get(read_json(\"config.json\"), name)'

The \"config.json\" file will never be read/parsed more than once and will not
be read/parsed once per row.
";

    colorize_cheatsheet(help)
}

pub fn get_moonblade_functions_help() -> String {
    let help = "
# Available functions & operators

(use --cheatsheet for a reminder of the expression language's basics)

## Operators

### Unary operators

    !x - boolean negation
    -x - numerical negation,

### Numerical comparison

Warning: those operators will always consider operands as numbers and will
try to cast them around as such. For string/sequence comparison, use the
operators in the next section.

    x == y - numerical equality
    x != y - numerical inequality
    x <  y - numerical less than
    x <= y - numerical less than or equal
    x >  y - numerical greater than
    x >= y - numerical greater than or equal

### String/sequence comparison

Warning: those operators will always consider operands as strings or
sequences and will try to cast them around as such. For numerical comparison,
use the operators in the previous section.

    x eq y - string equality
    x ne y - string inequality
    x lt y - string less than
    x le y - string less than or equal
    x gt y - string greater than
    x ge y - string greater than or equal

### Arithmetic operators

    x + y  - numerical addition
    x - y  - numerical subtraction
    x * y  - numerical multiplication
    x / y  - numerical division
    x % y  - numerical remainder

    x // y - numerical integer division
    x ** y - numerical exponentiation

## String operators

    x . y - string concatenation

## Logical operators

    x &&  y - logical and
    x and y
    x ||  y - logical or
    x or  y

    x in y
    x not in y

## Indexing & slicing operators

    x[y] - get y from x (string or list index, map key)
    x[start:end] - slice x from start index to end index
    x[:end] - slice x from start to end index
    x[start:] - slice x from start index to end

    Negative indices are accepted and mean the same thing as with
    the Python language.

## Pipeline operator (using \"_\" for left-hand side substitution)

    trim(name) | len(_)         - Same as len(trim(name))
    trim(name) | len            - Supports elision for unary functions
    trim(name) | add(1, len(_)) - Can be nested
    add(trim(name) | len, 2)    - Can be used anywhere

## Arithmetics

    - abs(x) -> number
        Return absolute value of number.

    - add(x, y, *n) -> number
        Add two or more numbers.

    - argmax(numbers, labels?) -> any
        Return the index or label of the largest number in the list.

    - argmin(numbers, labels?) -> any
        Return the index or label of the smallest number in the list.

    - ceil(x) -> number
        Return the smallest integer greater than or equal to x.

    - div(x, y, *n) -> number
        Divide two or more numbers.

    - floor(x) -> number
        Return the smallest integer lower than or equal to x.

    - idiv(x, y) -> number
        Integer division of two numbers.

    - log(x) -> number
        Return the natural logarithm of x.

    - max(x, y, *n) -> number
    - max(list_of_numbers) -> number
        Return the maximum number.

    - min(x, y, *n) -> number
    - min(list_of_numbers) -> number
        Return the minimum number.

    - mod(x, y) -> number
        Return the remainder of x divided by y.

    - mul(x, y, *n) -> number
        Multiply two or more numbers.

    - neg(x) -> number
        Return -x.

    - pow(x, y) -> number
        Raise x to the power of y.

    - round(x) -> number
        Return x rounded to the nearest integer.

    - sqrt(x) -> number
        Return the square root of x.

    - sub(x, y, *n) -> number
        Subtract two or more numbers.

    - trunc(x) -> number
        Truncate the number by removing its decimal part.

## Boolean operations & branching

    - and(a, b, *x) -> T
        Perform boolean AND operation on two or more values.

    - if(cond, then, else?) -> T
        Evaluate condition and switch to correct branch.
        Will actually short-circuit. Contrary to \"or\" and \"and\".

    - unless(cond, then, else?) -> T
        Shorthand for `if(not(cond), then, else?)`.

    - not(a) -> bool
        Perform boolean NOT operation.

    - or(a, b, *x) -> T
        Perform boolean OR operation on two or more values.

## Comparison

    - eq(s1, s2) -> bool
        Test string or sequence equality.

    - ne(s1, s2) -> bool
        Test string or sequence inequality.

    - gt(s1, s2) -> bool
        Test that string or sequence s1 > s2.

    - ge(s1, s2) -> bool
        Test that string or sequence s1 >= s2.

    - lt(s1, s2) -> bool
        Test that string or sequence s1 < s2.

    - ge(s1, s2) -> bool
        Test that string or sequence s1 <= s2.

## String & sequence helpers

    - compact(list) -> list
        Drop all falsey values from given list.

    - concat(string, *strings) -> string
        Concatenate given strings into a single one.

    - contains(seq, subseq) -> bool
        Find if subseq can be found in seq. Subseq can
        be a regular expression.

    - count(seq, pattern) -> int
        Count number of times pattern appear in seq. Pattern
        can be a regular expression.

    - endswith(string, pattern) -> bool
        Test if string ends with pattern.

    - escape_regex(string) -> string
        Escape a string so it can be used safely in a regular expression.

    - first(seq) -> T
        Get first element of sequence.

    - fmt(string, *replacements) -> string:
        Format a string by replacing \"{}\" occurrences by subsequent
        arguments.

        Example: `fmt(\"Hello {} {}\", name, surname)` will replace
        the first \"{}\" by the value of the name column, then the
        second one by the value of the surname column.

    - get(target, index_or_key, default?) -> T
        Get nth element of sequence (can use negative indexing), or key of mapping.
        Returns nothing if index or key is not found or alternatively the provided
        default value.

    - join(seq, sep) -> string
        Join sequence by separator.

    - last(seq) -> T
        Get last element of sequence.

    - len(seq) -> int
        Get length of sequence.

    - ltrim(string, pattern?) -> string
        Trim string of leading whitespace or
        provided characters.

    - lower(string) -> string
        Lowercase string.

    - match(string, pattern, group?) -> string
        Return a regex pattern match on the string.

    - numfmt(number) -> string:
        Format a number with thousands separator and proper significance.

    - replace(string, pattern, replacement) -> string
        Replace pattern in string. Can use a regex.

    - rtrim(string, pattern?) -> string
        Trim string of trailing whitespace or
        provided characters.

    - slice(seq, start, end?) -> seq
        Return slice of sequence.

    - split(string, sep, max?) -> list
        Split a string by separator.

    - startswith(string, pattern) -> bool
        Test if string starts with pattern.

    - trim(string, pattern?) -> string
        Trim string of leading & trailing whitespace or
        provided characters.

    - unidecode(string) -> string
        Convert string to ascii as well as possible.

    - upper(string) -> string
        Uppercase string.

## Dates

    - datetime(string, format=?, timezone=?) -> datetime
        Parse a string as a datetime according to format and timezone
        (https://docs.rs/jiff/latest/jiff/fmt/strtime/index.html#conversion-specifications).
        If no format is provided, string is parsed as ISO 8601 date format.
        Default timezone is the system timezone.

    - strftime(target, format, timezone=?) -> string
        Format target (a time in ISO 8601 format,
        or the result of datetime() function) according to format.

    - timestamp(number) -> datetime
        Parse a number as a POSIX timestamp in seconds
        (nb of seconds since 1970-01-01 00:00:00 UTC),
        and convert it to a datetime in local time.

    - timestamp_ms(number) -> datetime
        Parse a number as a POSIX timestamp in milliseconds
        (nb of milliseconds since 1970-01-01 00:00:00 UTC),
        and convert it to a datetime in local time.

    - year_month_day(target, timezone=?) -> string
    - ymd(target, timezone=?) -> string
        Extract the year, month and day of a datetime.
        If the input is a string, first parse it into datetime, and then extract the year, month and day.
        Equivalent to strftime(string, format = \"%Y-%m-%d\")

    - month_day(target, timezone=?) -> string
        Extract the month and day of a datetime.
        If the input is a string, first parse it into datetime, and then extract the month and day.
        Equivalent to strftime(string, format = \"%m-%d\")

    - month(target, timezone=?) -> string
        Extract the month of a datetime.
        If the input is a string, first parse it into datetime, and then extract the month.
        Equivalent to strftime(string, format = \"%m\")

    - year(target, timezone=?) -> string
        Extract the year of a datetime.
        If the input is a string, first parse it into datetime, and then extract the year.
        Equivalent to strftime(string, format = \"%Y\")

    - year_month(target, timezone=?) -> string
    - ym(target, timezone=?) -> string
        Extract the year and month of a datetime.
        If the input is a string, first parse it into datetime, and then extract the year and month.
        Equivalent to strftime(string, format = \"%Y-%m\")

## Collections (list of maps) functions

    - index_by(collection, key) -> map
        Create a map from item key to collection item.

## Map functions

    - keys(map) -> [string]
        Return a list of the map's keys.

    - values(map) -> [T]
        Return a list of the map's values.

## List aggregation functions

    - mean(numbers) -> number?
        Return the means of the given numbers.

## Fuzzy matching & information retrieval

    - fingerprint(string) -> string
        Fingerprint a string by normalizing characters, re-ordering
        and deduplicating its word tokens before re-joining them by
        spaces.

    - carry_stemmer(string) -> string
        Apply the \"Carry\" stemmer targeting the French language.

    - s_stemmer(string) -> string
        Apply a very simple stemmer removing common plural inflexions in
        some languages.

## Utils

    - coalesce(*args) -> T
        Return first truthy value.

    - col(name_or_pos, nth?) -> string
        Return value of cell for given column, by name, by position or by
        name & nth, in case of duplicate header names.

    - cols(from_name_or_pos?, to_name_or_pos?) -> list
        Return list of cell values from the given colum by name or position
        to another given column by name or position, inclusive.
        Can also be called with a single argument to take a slice from the
        given column to the end, or no argument at all to take all columns.

    - err(msg) -> error
        Make the expression return a custom error.

    - headers(from_name_or_pos?, to_name_or_pos?) -> list
        Return list of header names from the given colum by name or position
        to another given column by name or position, inclusive.
        Can also be called with a single argument to take a slice from the
        given column to the end, or no argument at all to return all headers.

    - index() -> integer?
        Return the row's index, if applicable.

    - json_parse(string) -> any
        Parse the given string as JSON.

    - typeof(value) -> string
        Return type of value.

## IO & path wrangling

    - abspath(string) -> string
        Return absolute & canonicalized path.

    - bytesize(integer) -> string
        Return a number of bytes in human-readable format (KB, MB, GB, etc.).

    - copy(source_path, target_path) -> string
        Copy a source to target path. Will create necessary directories
        on the way. Returns target path as a convenience.

    - ext(path) -> string?
        Return the path's extension, if any.

    - filesize(string) -> int
        Return the size of given file in bytes.

    - isfile(string) -> bool
        Return whether the given path is an existing file on disk.

    - move(source_path, target_path) -> string
        Move a source to target path. Will create necessary directories
        on the way. Returns target path as a convenience.

    - pathjoin(string, *strings) -> string
        Join multiple paths correctly.

    - read(path, encoding=?, errors=?) -> string
        Read file at path. Default encoding is \"utf-8\".
        Default error handling policy is \"replace\", and can be
        one of \"replace\", \"ignore\" or \"strict\".

    - read_csv(path) -> list[map]
        Read and parse CSV file at path, returning its rows as
        a list of maps with headers as keys.

    - read_json(path) -> any
        Read and parse JSON file at path.

    - write(string, path) -> string
        Write string to path as utf-8 text. Will create necessary
        directories recursively before actually writing the file.
        Return the path that was written.

## Random

    - md5(string) -> string
        Return the md5 hash of string in hexadecimal representation.

    - random() -> float
        Return a random float between 0 and 1.

    - uuid() -> string
        Return a uuid v4.

";

    colorize_functions_help(help)
}

pub fn get_moonblade_aggregations_function_help() -> String {
    let help = "
# Available aggregation functions

(use --cheatsheet for a reminder of how the scripting language works)

Note that most functions ignore null values (empty strings), but that functions
operating on numbers will yield an error if encountering a string that cannot
be safely parsed as a number.

You can always use `coalesce` to nudge values around and force aggregation functions to
consider null values or make them avoid non-numerical values altogether.

Example: considering null values when computing a mean => 'mean(coalesce(number, 0))'

    - all(<expr>) -> bool
        Returns true if all elements returned by given expression are truthy.

    - any(<expr>) -> bool
        Returns true if one of the elements returned by given expression is truthy.

    - approx_cardinality(<expr>) -> int
        Returns the approximate cardinality of the set of values returned by given
        expression using the HyperLogLog+ algorithm.

    - argmin(<expr>, <expr>?) -> any
        Return the index of the row where the first expression is minimized, or
        the result of the second expression where the first expression is minimized.
        Ties will be broken by original row index.

    - argmax(<expr>, <expr>?) -> any
        Return the index of the row where the first expression is maximized, or
        the result of the second expression where the first expression is maximized.
        Ties will be broken by original row index.

    - argtop(k, <expr>, <expr>?, separator?) -> string
        Find the top k values returned by the first expression and either
        return the indices of matching rows or the result of the second
        expression, joined by a pipe character ('|') or by the provided separator.
        Ties will be broken by original row index.

    - avg(<expr>) -> number
        Average of numerical values. Same as `mean`.

    - cardinality(<expr>) -> number
        Number of distinct values returned by given expression.

    - count(<expr>?) -> number
        Count the number of truthy values returned by given expression.
        Expression can also be omitted to count all rows.

    - count_seconds(<expr>) -> number
        Count the number of seconds between earliest and latest datetime
        returned by given expression.

    - count_hours(<expr>) -> number
        Count the number of hours between earliest and latest datetime
        returned by given expression.

    - count_days(<expr>) -> number
        Count the number of days between earliest and latest datetime
        returned by given expression.

    - count_years(<expr>) -> number
        Count the number of years between earliest and latest datetime
        returned by given expression.

    - distinct_values(<expr>, separator?) -> string
        List of sorted distinct values joined by a pipe character ('|') by default or by
        the provided separator.

    - earliest(<expr>) -> datetime
        Earliest datetime returned by given expression.

    - first(<expr>) -> string
        Return first seen non empty element of the values returned by the given expression.

    - latest(<expr>) -> datetime
        Latest datetime returned by given expression.

    - last(<expr>) -> string
        Return last seen non empty element of the values returned by the given expression.

    - lex_first(<expr>) -> string
        Return first string in lexicographical order.

    - lex_last(<expr>) -> string
        Return last string in lexicographical order.

    - min(<expr>) -> number | string
        Minimum numerical value.

    - max(<expr>) -> number | string
        Maximum numerical value.

    - mean(<expr>) -> number
        Mean of numerical values. Same as `avg`.

    - median(<expr>) -> number
        Median of numerical values, interpolating on even counts.

    - median_high(<expr>) -> number
        Median of numerical values, returning higher value on even counts.

    - median_low(<expr>) -> number
        Median of numerical values, returning lower value on even counts.

    - mode(<expr>) -> string
        Value appearing the most, breaking ties arbitrarily in favor of the
        first value in lexicographical order.

    - most_common(k, <expr>, separator?) -> string
        List of top k most common values returned by expression
        joined by a pipe character ('|') or by the provided separator.
        Ties will be broken by lexicographical order.

    - most_common_counts(k, <expr>, separator?) -> numbers
        List of top k most common counts returned by expression
        joined by a pipe character ('|') or by the provided separator.

    - percentage(<expr>) -> number
        Return the percentage of truthy values returned by expression.

    - quantile(<expr>, p) -> number
        Return the desired quantile of numerical values.

    - q1(<expr>) -> number
        Return the first quartile of numerical values.

    - q2(<expr>) -> number
        Return the second quartile of numerical values. Alias for median.

    - q3(<expr>) -> number
        Return the third quartile of numerical values.

    - ratio(<expr>) -> number
        Return the ratio of truthy values returned by expression.

    - stddev(<expr>) -> number
        Population standard deviation. Same as `stddev_pop`.

    - stddev_pop(<expr>) -> number
        Population standard deviation. Same as `stddev`.

    - stddev_sample(<expr>) -> number
        Sample standard deviation (i.e. using Bessel's correction).

    - sum(<expr>) -> number
        Sum of numerical values. Will return nothing if the sum overflows.
        Uses the Kahan-Babuska routine for precise float summation.

    - top(k, <expr>, separator?) -> any
        Find the top k values returned by the expression and join
        them by a pipe character ('|') or by the provided separator.
        Ties will be broken by original row index.

    - type(<expr>) -> string
        Best type description for seen values.

    - types(<expr>) -> string
        Sorted list, pipe-separated, of all the types seen in the values.

    - values(<expr>, separator?) -> string
        List of values joined by a pipe character ('|') by default or by
        the provided separator.

    - var(<expr>) -> number
        Population variance. Same as `var_pop`.

    - var_pop(<expr>) -> number
        Population variance. Same as `var`.

    - var_sample(<expr>) -> number
        Sample variance (i.e. using Bessel's correction).
";

    colorize_functions_help(help)
}

pub enum MoonbladeMode {
    Map,
    Foreach,
    Filter(bool),
    Transform,
    Flatmap,
}

impl MoonbladeMode {
    fn is_map(&self) -> bool {
        matches!(self, Self::Map)
    }

    fn is_flatmap(&self) -> bool {
        matches!(self, Self::Flatmap)
    }

    fn is_transform(&self) -> bool {
        matches!(self, Self::Transform)
    }

    fn cannot_report(&self) -> bool {
        matches!(self, Self::Filter(_) | Self::Flatmap | Self::Foreach)
    }
}

pub enum MoonbladeErrorPolicy {
    Panic,
    Report,
    Ignore,
    Log,
}

impl MoonbladeErrorPolicy {
    pub fn try_from_restricted(value: &str) -> Result<Self, CliError> {
        Ok(match value {
            "panic" => Self::Panic,
            "ignore" => Self::Ignore,
            "log" => Self::Log,
            _ => {
                return Err(CliError::Other(format!(
                    "unknown error policy \"{}\"",
                    value
                )))
            }
        })
    }

    fn will_report(&self) -> bool {
        matches!(self, Self::Report)
    }

    pub fn handle_row_error(
        &self,
        index: usize,
        error: SpecifiedEvaluationError,
    ) -> Result<(), SpecifiedEvaluationError> {
        match self {
            MoonbladeErrorPolicy::Panic => Err(error)?,
            MoonbladeErrorPolicy::Ignore => Ok(()),
            MoonbladeErrorPolicy::Log => {
                eprintln!("Row n°{}: {}", index, error);
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    pub fn handle_error<T: Default>(
        &self,
        result: Result<T, SpecifiedEvaluationError>,
    ) -> Result<T, SpecifiedEvaluationError> {
        match result {
            Ok(value) => Ok(value),
            Err(err) => match self {
                MoonbladeErrorPolicy::Panic => Err(err)?,
                MoonbladeErrorPolicy::Ignore => Ok(T::default()),
                MoonbladeErrorPolicy::Log => {
                    eprintln!("{}", err);
                    Ok(T::default())
                }
                _ => unreachable!(),
            },
        }
    }
}

impl TryFrom<String> for MoonbladeErrorPolicy {
    type Error = CliError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(match value.as_str() {
            "panic" => Self::Panic,
            "report" => Self::Report,
            "ignore" => Self::Ignore,
            "log" => Self::Log,
            _ => {
                return Err(CliError::Other(format!(
                    "unknown error policy \"{}\"",
                    value
                )))
            }
        })
    }
}

pub struct MoonbladeCmdArgs {
    pub print_cheatsheet: bool,
    pub print_functions: bool,
    pub target_column: Option<String>,
    pub rename_column: Option<String>,
    pub map_expr: String,
    pub input: Option<String>,
    pub output: Option<String>,
    pub no_headers: bool,
    pub delimiter: Option<Delimiter>,
    pub parallelization: Option<Option<usize>>,
    pub error_policy: MoonbladeErrorPolicy,
    pub error_column_name: Option<String>,
    pub mode: MoonbladeMode,
}

pub fn handle_eval_result<'b>(
    args: &MoonbladeCmdArgs,
    index: usize,
    record: &'b mut csv::ByteRecord,
    eval_result: Result<DynamicValue, SpecifiedEvaluationError>,
    replace: Option<usize>,
) -> Result<Vec<Cow<'b, csv::ByteRecord>>, String> {
    let mut records_to_emit: Vec<Cow<csv::ByteRecord>> = Vec::new();

    match eval_result {
        Ok(value) => match args.mode {
            MoonbladeMode::Filter(invert) => {
                let mut should_emit = value.is_truthy();

                if invert {
                    should_emit = !should_emit;
                }

                if should_emit {
                    records_to_emit.push(Cow::Borrowed(record));
                }
            }
            MoonbladeMode::Map => {
                record.push_field(&value.serialize_as_bytes());

                if args.error_policy.will_report() {
                    record.push_field(b"");
                }

                records_to_emit.push(Cow::Borrowed(record));
            }
            MoonbladeMode::Foreach => {}
            MoonbladeMode::Transform => {
                let mut record = record.replace_at(replace.unwrap(), &value.serialize_as_bytes());

                if args.error_policy.will_report() {
                    record.push_field(b"");
                }

                records_to_emit.push(Cow::Owned(record));
            }
            MoonbladeMode::Flatmap => 'm: {
                if value.is_falsey() {
                    break 'm;
                }

                for subvalue in value.flat_iter() {
                    let cell = subvalue.serialize_as_bytes();

                    let new_record = if let Some(idx) = replace {
                        record.replace_at(idx, &cell)
                    } else {
                        record.append(&cell)
                    };

                    records_to_emit.push(Cow::Owned(new_record));
                }
            }
        },
        Err(err) => match args.error_policy {
            MoonbladeErrorPolicy::Ignore => {
                if args.mode.is_map() {
                    record.push_field(b"");
                    records_to_emit.push(Cow::Borrowed(record));
                } else if args.mode.is_transform() {
                    let record = record.replace_at(replace.unwrap(), b"");
                    records_to_emit.push(Cow::Owned(record));
                }
            }
            MoonbladeErrorPolicy::Report => {
                if args.mode.cannot_report() {
                    unreachable!();
                }

                if args.mode.is_map() {
                    record.push_field(b"");
                    record.push_field(err.to_string().as_bytes());
                    records_to_emit.push(Cow::Borrowed(record));
                } else if args.mode.is_transform() {
                    let mut record = record.replace_at(replace.unwrap(), b"");
                    record.push_field(err.to_string().as_bytes());
                    records_to_emit.push(Cow::Owned(record));
                }
            }
            MoonbladeErrorPolicy::Log => {
                eprintln!("Row n°{}: {}", index + 1, err);

                if args.mode.is_map() {
                    record.push_field(b"");
                    records_to_emit.push(Cow::Borrowed(record));
                } else if args.mode.is_transform() {
                    let record = record.replace_at(replace.unwrap(), b"");
                    records_to_emit.push(Cow::Owned(record));
                }
            }
            MoonbladeErrorPolicy::Panic => {
                return Err(format!("Row n°{}: {}", index + 1, err));
            }
        },
    };

    Ok(records_to_emit)
}

pub fn run_moonblade_cmd(args: MoonbladeCmdArgs) -> CliResult<()> {
    if args.print_cheatsheet {
        println!("{}", get_moonblade_cheatsheet());
        return Ok(());
    }

    if args.print_functions {
        println!("{}", get_moonblade_functions_help());
        return Ok(());
    }

    let mut rconfig = Config::new(&args.input)
        .delimiter(args.delimiter)
        .no_headers(args.no_headers);

    let mut rdr = rconfig.reader()?;
    let mut wtr = Config::new(&args.output).writer()?;

    let mut headers = csv::ByteRecord::new();
    let mut modified_headers = csv::ByteRecord::new();
    let mut must_write_headers = false;
    let mut column_to_replace: Option<usize> = None;
    let mut map_expr = args.map_expr.clone();

    if !args.no_headers {
        headers = rdr.byte_headers()?.clone();
        modified_headers = headers.clone();

        if !headers.is_empty() {
            must_write_headers = true;

            if args.mode.is_map() {
                if let Some(target_column) = &args.target_column {
                    modified_headers.push_field(target_column.as_bytes());
                }
            } else if args.mode.is_transform() {
                if let Some(name) = &args.target_column {
                    rconfig = rconfig.select(SelectColumns::parse(name)?);
                    let idx = rconfig.single_selection(&headers)?;

                    if let Some(renamed) = &args.rename_column {
                        modified_headers = modified_headers.replace_at(idx, renamed.as_bytes());
                    }

                    column_to_replace = Some(idx);

                    // NOTE: binding implicit last value to target column value
                    map_expr = format!("col({}) | {}", idx, map_expr);
                }
            } else if args.mode.is_flatmap() {
                if let Some(replaced) = &args.rename_column {
                    rconfig = rconfig.select(SelectColumns::parse(replaced)?);
                    let idx = rconfig.single_selection(&headers)?;

                    if let Some(renamed) = &args.target_column {
                        modified_headers = modified_headers.replace_at(idx, renamed.as_bytes());
                    }

                    column_to_replace = Some(idx);
                } else if let Some(target_column) = &args.target_column {
                    modified_headers.push_field(target_column.as_bytes());
                }
            }

            if args.error_policy.will_report() {
                if let Some(error_column_name) = &args.error_column_name {
                    modified_headers.push_field(error_column_name.as_bytes());
                }
            }
        }
    }

    let program = Program::parse(&map_expr, &headers)?;

    if must_write_headers {
        wtr.write_byte_record(&modified_headers)?;
    }

    if let Some(threads) = args.parallelization {
        rdr.into_byte_records()
            .enumerate()
            .parallel_map_custom(
                |o| {
                    if let Some(count) = threads {
                        o.threads(count)
                    } else {
                        o
                    }
                },
                move |(i, record)| -> CliResult<(
                    usize,
                    csv::ByteRecord,
                    Result<DynamicValue, SpecifiedEvaluationError>,
                )> {
                    let record = record?;

                    let eval_result = program.run_with_record(i, &record);

                    Ok((i, record, eval_result))
                },
            )
            .try_for_each(|result| -> CliResult<()> {
                let (i, mut record, eval_result) = result?;
                let records_to_emit =
                    handle_eval_result(&args, i, &mut record, eval_result, column_to_replace)?;

                for record_to_emit in records_to_emit {
                    wtr.write_byte_record(&record_to_emit)?;
                }
                Ok(())
            })?;

        return Ok(wtr.flush()?);
    }

    let mut record = csv::ByteRecord::new();
    let mut i: usize = 0;

    while rdr.read_byte_record(&mut record)? {
        let eval_result = program.run_with_record(i, &record);

        let records_to_emit =
            handle_eval_result(&args, i, &mut record, eval_result, column_to_replace)?;

        for record_to_emit in records_to_emit {
            wtr.write_byte_record(&record_to_emit)?;
        }

        i += 1;
    }

    Ok(wtr.flush()?)
}
