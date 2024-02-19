use std::cell::RefCell;
use std::sync::Arc;

use csv;
use rayon::prelude::*;
use thread_local::ThreadLocal;

use config::{Config, Delimiter};
use util;
use CliResult;

use moonblade::AggregationProgram;

use cmd::moonblade::{
    get_moonblade_aggregations_function_help, get_moonblade_cheatsheet,
    get_moonblade_functions_help, MoonbladeErrorPolicy,
};

static USAGE: &str = "
Aggregate CSV data using a custom aggregation expression. The result of running
the command will be a single row of CSV containing the result of aggregating
the whole file.

You can, for instance, compute the sum of a column:

    $ xan agg 'sum(retweet_count)' file.csv

You can use dynamic expressions to mangle the data before aggregating it:

    $ xan agg 'sum(retweet_count + replies_count)' file.csv

You can perform multiple aggregations at once:

    $ xan agg 'sum(retweet_count), mean(retweet_count), max(replies_count)' file.csv

You can rename the output columns using the 'as' syntax:

    $ xan agg 'sum(n) as sum, max(replies_count) as \"Max Replies\"' file.csv

For a quick review of the capabilities of the script language, use
the --cheatsheet flag.

For a list of available aggregation functions, use the --aggs flag.

If you want to list available functions, use the --functions flag.

Usage:
    xan agg [options] <expression> [<input>]
    xan agg --help
    xan agg --cheatsheet
    xan agg --aggs
    xan agg --functions

agg options:
    -e, --errors <policy>   What to do with evaluation errors. One of:
                              - \"panic\": exit on first error
                              - \"ignore\": ignore row altogether
                              - \"log\": print error to stderr
                            [default: panic].
    -p, --parallel          Whether to use parallelization to speed up computations.
                            Will automatically select a suitable number of threads to use
                            based on your number of cores.

Common options:
    -h, --help               Display this message
    -o, --output <file>      Write output to <file> instead of stdout.
    -n, --no-headers         When set, the first row will not be evaled
                             as headers.
    -d, --delimiter <arg>    The field delimiter for reading CSV data.
                             Must be a single character. [default: ,]
";

#[derive(Deserialize)]
struct Args {
    arg_expression: String,
    arg_input: Option<String>,
    flag_no_headers: bool,
    flag_output: Option<String>,
    flag_delimiter: Option<Delimiter>,
    flag_aggs: bool,
    flag_errors: String,
    flag_cheatsheet: bool,
    flag_functions: bool,
    flag_parallel: bool,
}

pub fn run(argv: &[&str]) -> CliResult<()> {
    let args: Args = util::get_args(USAGE, argv)?;

    if args.flag_aggs {
        println!("{}", get_moonblade_aggregations_function_help());
        return Ok(());
    }

    if args.flag_cheatsheet {
        println!("{}", get_moonblade_cheatsheet());
        return Ok(());
    }

    if args.flag_functions {
        println!("{}", get_moonblade_functions_help());
        return Ok(());
    }

    let error_policy = MoonbladeErrorPolicy::from_restricted(&args.flag_errors)?;

    let rconf = Config::new(&args.arg_input)
        .delimiter(args.flag_delimiter)
        .no_headers(args.flag_no_headers);

    let mut rdr = rconf.reader()?;
    let mut wtr = Config::new(&args.flag_output).writer()?;
    let headers = rdr.byte_headers()?;

    let mut program = AggregationProgram::parse(&args.arg_expression, headers)?;

    wtr.write_record(program.headers())?;

    if !args.flag_parallel {
        let mut record = csv::ByteRecord::new();
        let mut index: usize = 0;

        while rdr.read_byte_record(&mut record)? {
            index += 1;

            program
                .run_with_record(index, &record)
                .or_else(|error| error_policy.handle_error(index, error))?;
        }
    } else {
        let local: Arc<ThreadLocal<RefCell<AggregationProgram>>> = Arc::new(ThreadLocal::new());

        rdr.into_byte_records()
            .enumerate()
            .par_bridge()
            .try_for_each(|(index, rdr_result)| -> CliResult<()> {
                let record = rdr_result?;

                let mut local_program = local.get_or(|| RefCell::new(program.clone())).borrow_mut();

                local_program
                    .run_with_record(index, &record)
                    .or_else(|error| error_policy.handle_error(index, error))?;

                Ok(())
            })?;

        for local_program in Arc::try_unwrap(local).unwrap().into_iter() {
            program.merge(local_program.into_inner());
        }
    }

    wtr.write_byte_record(&program.finalize(args.flag_parallel))?;

    Ok(wtr.flush()?)
}
