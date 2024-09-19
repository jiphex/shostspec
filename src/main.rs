use std::{env::args, num::ParseIntError, ops::RangeInclusive, process::exit, vec};

#[derive(Debug)]
struct HostSpec {
    numeric: RangeInclusive<u64>,
    prefix: String,
}

impl Iterator for HostSpec {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.numeric.next().map(|n| format!("{}{}", self.prefix, n))
    }
}

impl HostSpec {
    fn from_single(host: &str) -> Result<Self, ParseError> {
        let prefix = host.trim_end_matches(|c: char| c.is_ascii_digit());
        // unwrap here, but stripping what we know is the first part of the string, from the string, should never fail
        let numeric_part = host.strip_prefix(prefix).unwrap();
        let host_number = numeric_part.parse::<u64>()?;
        Ok(HostSpec {
            numeric: host_number..=host_number,
            prefix: prefix.to_string(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum ParseError {
    #[error("the expression contained a spec with unknown extra characters (e.g after the closing ']' character)")]
    ExtraStuff,
    #[error("the expression contained a spec with numbers that couldn't be understood, or no numbers at all")]
    BadNumbers(#[from] ParseIntError),
    #[error("the expression contained a spec that looked like a range[numbers], but was badly formed")]
    NoRange,
}

fn transform_numeric_range(range_str: &str) -> Result<RangeInclusive<u64>, ParseIntError> {
    if let Some((start, end)) = range_str.split_once("-") {
        let start_n = start.parse()?;
        let end_n = end.parse()?;
        Ok(start_n..=(end_n))
    } else {
        let single_int = range_str.parse()?;
        Ok(single_int..=single_int)
    }
}

fn transform_single_hostspec(item: impl AsRef<str>) -> Result<Vec<HostSpec>, ParseError> {
    let raw: &str = item.as_ref();
    if let Some((prefix, rangesuffix)) = raw.split_once("[") {
        if let Some((range, suffix)) = rangesuffix.split_once("]") {
            if !suffix.is_empty() {
                Err(ParseError::ExtraStuff)
            } else {
                range
                    .split(',')
                    .map(transform_numeric_range)
                    .map(|range| match range {
                        Ok(range) => Ok(HostSpec {
                            prefix: prefix.into(),
                            numeric: range,
                        }),
                        Err(e) => Err(ParseError::BadNumbers(e)),
                    })
                    .collect() // Because Rust is magic, this will check that the iterator is all Ok(...), and convert it into a Vec of the Ok results, or short-circuit at the first Err(...) and return that instead
            }
        } else {
            Err(ParseError::NoRange)
        }
    } else {
        // Convert the single hostspec into a vec of hostspec, or just pass on the err
        HostSpec::from_single(raw).map(|i| vec![i])
    }
}

fn main() {
    args()
        .enumerate() // enumerate (with numeric indexes) the args, so we can use the index for error output
        .skip(1) // skip the first arg, the executable name
        .filter(|(_, s)| !s.is_empty()) // skip any empty args (not sure if these are even possible, because shell?)
        .map(|(n, arg)| (n, transform_single_hostspec(arg))) // map each (inner, unenumerated) arg into a vec of hostspecs
        .flat_map(|(n, m)| {
            // stop on the first error (flattening as we go)
            if let Err(e) = m {
                eprintln!("error at arg {n}: {}", e);
                exit(1)
            } else {
                m
            }
        })
        .flatten() // flatten each _arg_ to single hostspecs (some single _args_ may contain many _hostspecs_ - e.g host[1-10,20-30] would contain 2 hostspecs)
        .flatten() // flatten each _hostspec_ into an array of strings (to cover ranges, e.g host[1-10] would make 10 strings)
        .for_each(|h| println!("{}", h)); // finally, print each single host, on a single line
}
