extern crate arrayvec;
extern crate atty;
extern crate byteorder;
extern crate bytesize;
extern crate calamine;
extern crate colored;
extern crate console;
extern crate crossbeam_channel;
extern crate csv;
extern crate csv_index;
extern crate ctrlc;
extern crate docopt;
extern crate emojis;
extern crate encoding;
extern crate ext_sort;
extern crate filetime;
extern crate flate2;
extern crate glob;
extern crate indicatif;
extern crate rand_chacha;
extern crate rand_seeder;
extern crate ratatui;
#[macro_use]
extern crate lazy_static;
extern crate num_cpus;
extern crate numfmt;
#[cfg(not(windows))]
extern crate pager;
extern crate pariter;
extern crate rand;
extern crate rayon;
extern crate regex;
extern crate serde;
extern crate thread_local;
#[macro_use]
extern crate serde_derive;
extern crate pest;
extern crate pest_derive;
extern crate serde_json;
extern crate termsize;
extern crate textwrap;
extern crate threadpool;
extern crate unicode_segmentation;
extern crate unicode_width;
extern crate unidecode;
extern crate uuid;

use std::borrow::ToOwned;
use std::env;
use std::fmt;
use std::io;
use std::process;

use docopt::Docopt;

macro_rules! wout {
    ($($arg:tt)*) => ({
        use std::io::Write;
        (writeln!(&mut ::std::io::stdout(), $($arg)*)).unwrap();
    });
}

macro_rules! werr {
    ($($arg:tt)*) => ({
        use std::io::Write;
        (writeln!(&mut ::std::io::stderr(), $($arg)*)).unwrap();
    });
}

macro_rules! fail {
    ($e:expr) => {
        Err(::std::convert::From::from($e))
    };
}

macro_rules! command_list {
    () => {
        "
    help        Show this usage message.

## Explore & visualize
    count       Count rows in file
    headers (h) Show header names
    view    (v) Preview a CSV file in a human-friendly way
    flatten (f) Display a flattened version of each row of a file
    hist        Print a histogram with rows of CSV file as bars
    plot        Draw a scatter plot or line chart
    progress    Display a progress bar while reading CSV data

## Search & filter
    search      Search CSV data with regexes
    filter      Only keep some CSV rows based on an evaluated expression
    slice       Slice rows of CSV file
    top         Find top rows of a CSV file according to some column
    sample      Randomly sample CSV data

## Sort & deduplicate
    sort        Sort CSV data
    dedup       Deduplicate a CSV file
    shuffle     Shuffle CSV data

## Combine multiple CSV files
    cat         Concatenate by row or column
    join        Join CSV files
    merge       Merge multiple similar already sorted CSV files

## Split a CSV file into multiple
    split       Split CSV data into chunks
    partition   Partition CSV data based on a column value

## Add & transform columns
    map         Create a new column by evaluating an expression on each CSV row
    transform   Transform a column by evaluating an expression on each CSV row
    enum        Enumerate CSV file by preprending an index column
    flatmap     Emit one row per value yielded by an expression evaluated for each CSV row

## Format, convert & recombobulate
    select      Select columns from CSV
    behead      Drop header from CSV file
    rename      Rename columns of a CSV file
    input       Read CSV data with special quoting rules
    fixlengths  Makes all rows have same length
    fmt         Format CSV output (change field delimiter)
    explode     Explode rows based on some column separator
    implode     Collapse consecutive identical rows based on a diverging column
    from        Convert a variety of formats to CSV
    reverse     Reverse rows of CSV data
    transpose   Transpose CSV file

## Aggregate
    frequency (freq) Show frequency tables
    groupby          Aggregate data by groups of a CSV file
    stats            Compute basic statistics
    agg              Aggregate data from CSV file
    bins             Dispatch numeric columns into bins

## Generate CSV files
    glob        Create a CSV file with paths matching a glob pattern
    range       Create a CSV file from a numerical range

## Perform side-effects
    foreach     Loop over a CSV file to perform side effects

## Lexicometry & fuzzy matching
    tokenize    Tokenize a text column
    vocab       Build a vocabulary over tokenized documents
    cluster     Cluster CSV data to find near-duplicates

## Graph algorithms
    union-find  Apply the union-find algorithm on a CSV edge list
"
    };
}

mod cmd;
mod collections;
mod config;
mod index;
mod json;
mod moonblade;
mod select;
mod util;

static USAGE: &str = concat!(
    "
Usage:
    xan <command> [<args>...]
    xan [options]

Options:
    --list        List all commands available.
    -h, --help    Display this message
    <command> -h  Display the command help message
    --version     Print version info and exit

Commands:",
    command_list!()
);

#[derive(Deserialize)]
struct Args {
    arg_command: Option<Command>,
    flag_list: bool,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| {
            d.options_first(true)
                .version(Some(util::version()))
                .deserialize()
        })
        .unwrap_or_else(|e| e.exit());
    if args.flag_list {
        wout!(concat!("Installed commands:", command_list!()));
        return;
    }
    match args.arg_command {
        None => {
            werr!(
                "{}",
                util::colorize_main_help(concat!(
                    "xan is a suite of CSV command line utilities.

Please choose one of the following commands:",
                    command_list!()
                ))
            );
            process::exit(0);
        }
        Some(cmd) => match cmd.run() {
            Ok(()) => process::exit(0),
            Err(CliError::Flag(err)) => err.exit(),
            Err(CliError::Csv(err)) => {
                werr!("{}", err);
                process::exit(1);
            }
            Err(CliError::Io(ref err)) if err.kind() == io::ErrorKind::BrokenPipe => {
                process::exit(0);
            }
            Err(CliError::Io(err)) => {
                werr!("{}", err);
                process::exit(1);
            }
            Err(CliError::Other(msg)) => {
                werr!("{}", msg);
                process::exit(1);
            }
        },
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Command {
    Agg,
    Behead,
    Bins,
    Blank,
    Cat,
    Cluster,
    Count,
    Dedup,
    Enum,
    Eval,
    Explode,
    ForEach,
    F,
    Filter,
    FixLengths,
    Flatmap,
    Flatten,
    Fmt,
    Freq,
    Frequency,
    From,
    Glob,
    Groupby,
    H,
    Headers,
    Help,
    Hist,
    Implode,
    Index,
    Input,
    Join,
    Map,
    Merge,
    Partition,
    Plot,
    Progress,
    Range,
    Rename,
    Reverse,
    Sample,
    Search,
    Select,
    Shuffle,
    Slice,
    Sort,
    Split,
    Stats,
    Tokenize,
    Top,
    Transform,
    Transpose,
    #[serde(rename = "union-find")]
    UnionFind,
    V,
    View,
    Vocab,
}

impl Command {
    fn run(self) -> CliResult<()> {
        let argv: Vec<_> = env::args().collect();
        let argv: Vec<_> = argv.iter().map(|s| &**s).collect();
        let argv = &*argv;

        if !argv[1].chars().all(|c| char::is_lowercase(c) || c == '-') {
            return Err(CliError::Other(format!(
                "xan expects commands in lowercase. Did you mean '{}'?",
                argv[1].to_lowercase()
            )));
        }
        match self {
            Command::Agg => cmd::agg::run(argv),
            Command::Behead => cmd::behead::run(argv),
            Command::Bins => cmd::bins::run(argv),
            Command::Blank => cmd::blank::run(argv),
            Command::Cat => cmd::cat::run(argv),
            Command::Cluster => cmd::cluster::run(argv),
            Command::Count => cmd::count::run(argv),
            Command::Dedup => cmd::dedup::run(argv),
            Command::Enum => cmd::enumerate::run(argv),
            Command::Eval => cmd::eval::run(argv),
            Command::Explode => cmd::explode::run(argv),
            Command::Filter => cmd::filter::run(argv),
            Command::FixLengths => cmd::fixlengths::run(argv),
            Command::Flatmap => cmd::flatmap::run(argv),
            Command::F => cmd::flatten::run(argv),
            Command::Flatten => cmd::flatten::run(argv),
            Command::Fmt => cmd::fmt::run(argv),
            Command::ForEach => cmd::foreach::run(argv),
            Command::Freq | Command::Frequency => cmd::frequency::run(argv),
            Command::From => cmd::from::run(argv),
            Command::Glob => cmd::glob::run(argv),
            Command::Groupby => cmd::groupby::run(argv),
            Command::H => cmd::headers::run(argv),
            Command::Headers => cmd::headers::run(argv),
            Command::Help => {
                wout!("{}", util::colorize_main_help(USAGE));
                Ok(())
            }
            Command::Hist => cmd::hist::run(argv),
            Command::Implode => cmd::implode::run(argv),
            Command::Index => cmd::index::run(argv),
            Command::Input => cmd::input::run(argv),
            Command::Join => cmd::join::run(argv),
            Command::Map => cmd::map::run(argv),
            Command::Merge => cmd::merge::run(argv),
            Command::Partition => cmd::partition::run(argv),
            Command::Plot => cmd::plot::run(argv),
            Command::Progress => cmd::progress::run(argv),
            Command::Range => cmd::range::run(argv),
            Command::Rename => cmd::rename::run(argv),
            Command::Reverse => cmd::reverse::run(argv),
            Command::Sample => cmd::sample::run(argv),
            Command::Search => cmd::search::run(argv),
            Command::Select => cmd::select::run(argv),
            Command::Shuffle => cmd::shuffle::run(argv),
            Command::Slice => cmd::slice::run(argv),
            Command::Sort => cmd::sort::run(argv),
            Command::Split => cmd::split::run(argv),
            Command::Stats => cmd::stats::run(argv),
            Command::Tokenize => cmd::tokenize::run(argv),
            Command::Top => cmd::top::run(argv),
            Command::Transform => cmd::transform::run(argv),
            Command::Transpose => cmd::transpose::run(argv),
            Command::UnionFind => cmd::union_find::run(argv),
            Command::V => cmd::view::run(argv),
            Command::View => cmd::view::run(argv),
            Command::Vocab => cmd::vocab::run(argv),
        }
    }
}

pub type CliResult<T> = Result<T, CliError>;

#[derive(Debug)]
pub enum CliError {
    Flag(docopt::Error),
    Csv(csv::Error),
    Io(io::Error),
    Other(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CliError::Flag(ref e) => e.fmt(f),
            CliError::Csv(ref e) => e.fmt(f),
            CliError::Io(ref e) => e.fmt(f),
            CliError::Other(ref s) => f.write_str(s),
        }
    }
}

impl From<docopt::Error> for CliError {
    fn from(err: docopt::Error) -> CliError {
        match err {
            docopt::Error::WithProgramUsage(_, usage) => {
                CliError::Other(util::colorize_help(&usage))
            }
            _ => CliError::Flag(err),
        }
    }
}

impl From<csv::Error> for CliError {
    fn from(err: csv::Error) -> CliError {
        if !err.is_io_error() {
            return CliError::Csv(err);
        }
        match err.into_kind() {
            csv::ErrorKind::Io(v) => From::from(v),
            _ => unreachable!(),
        }
    }
}

impl From<io::Error> for CliError {
    fn from(err: io::Error) -> CliError {
        CliError::Io(err)
    }
}

impl From<String> for CliError {
    fn from(err: String) -> CliError {
        CliError::Other(err)
    }
}

impl<'a> From<&'a str> for CliError {
    fn from(err: &'a str) -> CliError {
        CliError::Other(err.to_owned())
    }
}

impl From<regex::Error> for CliError {
    fn from(err: regex::Error) -> CliError {
        match err {
            regex::Error::CompiledTooBig(size) => {
                CliError::Other(format!("attempted to create too large a regex ({} bytes)! regexes are probably not the answer here, sorry :'(. did you forget to use the -e, --exact flag?", size))
            }
            _ => CliError::Other(format!("{:?}", err)),
        }
    }
}

impl From<calamine::Error> for CliError {
    fn from(err: calamine::Error) -> Self {
        CliError::Other(err.to_string())
    }
}

impl From<moonblade::ConcretizationError> for CliError {
    fn from(err: moonblade::ConcretizationError) -> CliError {
        CliError::Other(err.to_string())
    }
}

impl From<moonblade::EvaluationError> for CliError {
    fn from(err: moonblade::EvaluationError) -> CliError {
        CliError::Other(err.to_string())
    }
}

impl From<moonblade::SpecifiedEvaluationError> for CliError {
    fn from(err: moonblade::SpecifiedEvaluationError) -> CliError {
        CliError::Other(err.to_string())
    }
}

impl From<glob::GlobError> for CliError {
    fn from(err: glob::GlobError) -> Self {
        CliError::Other(err.to_string())
    }
}

impl From<glob::PatternError> for CliError {
    fn from(err: glob::PatternError) -> Self {
        CliError::Other(err.to_string())
    }
}

impl From<transient_btree_index::Error> for CliError {
    fn from(value: transient_btree_index::Error) -> Self {
        CliError::Other(value.to_string())
    }
}

impl From<()> for CliError {
    fn from(_: ()) -> CliError {
        CliError::Other("unknown error".to_string())
    }
}
