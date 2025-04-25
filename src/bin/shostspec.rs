use std::{num::ParseIntError, ops::RangeInclusive, process::exit, vec};

use clap::Parser;

#[derive(Debug, PartialEq)]
struct HostSpec {
    numeric: RangeInclusive<u64>,
    prefix: String,
    num_width_chars: Option<usize>,
}

impl Default for HostSpec {
    fn default() -> Self {
        Self {
            numeric: 0..=0,
            prefix: Default::default(),
            num_width_chars: None,
        }
    }
}

impl Iterator for HostSpec {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.numeric.next().map(|n| {
            format!(
                "{}{:0width$}", // This is some slightly odd syntax, but it delegates the length of the 0-padding to the kwarg
                self.prefix,
                n,
                width = self.num_width_chars.unwrap_or_default()
            )
        })
    }
}

impl HostSpec {
    fn from_single(host: &str) -> Result<Self, ParseError> {
        let prefix = host.trim_end_matches(|c: char| c.is_ascii_digit());
        // unwrap here, but stripping what we know is the first part of the string, from the string, should never fail
        let numeric_part = host.strip_prefix(prefix).unwrap();
        let host_number = numeric_part.parse::<u64>()?;
        let num_width_chars: Option<usize> =
            Some(numeric_part.chars().take_while(|c| c.is_numeric()).count());
        Ok(HostSpec {
            numeric: host_number..=host_number,
            prefix: prefix.to_string(),
            num_width_chars,
        })
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
enum ParseError {
    #[error("the expression contained a spec with unknown extra characters (e.g after the closing ']' character)")]
    ExtraStuff,
    #[error("the expression contained a spec with numbers that couldn't be understood, or no numbers at all")]
    BadNumbers(#[from] ParseIntError),
    #[error(
        "the expression contained a spec that looked like a range[numbers], but was badly formed (or reversed)"
    )]
    NoRange,
}

/// Convert a single range string consisting of two ascii digit numbers,
/// separated by a single hyphen character (e.g 123-456)
fn transform_numeric_range(range_str: &str) -> Result<RangeInclusive<u64>, ParseIntError> {
    match range_str.split_once("-") {
        Some((start, end)) => {
            let start_n = start.parse()?;
            let end_n = end.parse()?;
            Ok(start_n..=(end_n))
        }
        None => {
            let single_int = range_str.parse()?;
            Ok(single_int..=single_int)
        }
    }
}

fn transform_single_hostspec(item: impl AsRef<str>) -> Result<Vec<HostSpec>, ParseError> {
    let raw: &str = item.as_ref();
    match raw.split_once("[") {
        Some((prefix, rangesuffix)) => {
            match rangesuffix.split_once("]") {
                Some((range, suffix)) => {
                    if !suffix.is_empty() {
                        Err(ParseError::ExtraStuff)
                    } else {
                        let num_width_chars: Option<usize> =
                            Some(range.chars().take_while(|c| c.is_numeric()).count());
                        range
                            .split(',')
                            .map(transform_numeric_range)
                            .map(|range| match range {
                                Ok(range) => {
                                    if range.is_empty() {
                                        return Err(ParseError::NoRange);
                                    }
                                    Ok(HostSpec {
                                        prefix: prefix.into(),
                                        numeric: range,
                                        num_width_chars,
                                    })
                                }
                                Err(e) => Err(ParseError::BadNumbers(e)),
                            })
                            .collect() // Because Rust is magic, this will check that the iterator is all Ok(...), and convert it into a Vec of the Ok results, or short-circuit at the first Err(...) and return that instead
                    }
                }
                None => Err(ParseError::NoRange),
            }
        }
        None => {
            // Convert the single hostspec into a vec of hostspec, or just pass on the err
            HostSpec::from_single(raw).map(|i| vec![i])
        }
    }
}

#[derive(clap::Parser)]
struct CmdArgs {
    /// Any number of host specs, separated by whitespace e.g host[123]
    items: Vec<String>,
}

fn main() {
    let cla = CmdArgs::parse();
    cla.items
        .iter()
        .enumerate() // enumerate (with numeric indexes) the args, so we can use the index for error output
        .filter(|(_, s)| !s.is_empty()) // skip any empty args (not sure if these are even possible, because shell?)
        .map(|(n, arg)| (n, transform_single_hostspec(arg))) // map each (inner, unenumerated) arg into a vec of hostspecs
        .flat_map(|(n, m)| {
            // stop on the first error (flattening as we go)
            if let Err(e) = m {
                eprintln!("error at arg {n}: {}", e);
                exit(1)
            } else {
                // strip off the enumerated bit, we don't need it now, because its only used for highlighting errors, and we don't have any of those now
                m
            }
        })
        .flatten() // flatten each _arg_ to single hostspecs (some single _args_ may contain many _hostspecs_ - e.g host[1-10,20-30] would contain 2 hostspecs)
        .flatten() // flatten each _hostspec_ into an array of strings (to cover ranges, e.g host[1-10] would make 10 strings)
        .for_each(|h| println!("{}", h)); // finally, print each single host, on a single line
}

#[cfg(test)]
mod test {
    use crate::{transform_single_hostspec, HostSpec};

    #[test]
    fn test_basics() -> anyhow::Result<()> {
        assert_eq!(
            transform_single_hostspec("host[1234]"),
            Ok(vec![HostSpec {
                numeric: 1234..=1234,
                prefix: "host".into(),
                num_width_chars: Some(4),
            }])
        );
        assert_eq!(
            transform_single_hostspec("host[10-100,500]"),
            Ok(vec![
                HostSpec {
                    numeric: 10..=100,
                    prefix: "host".into(),
                    num_width_chars: Some(2),
                },
                HostSpec {
                    numeric: 500..=500,
                    prefix: "host".into(),
                    num_width_chars: Some(2),
                }
            ])
        );
        assert_eq!(
            transform_single_hostspec("xxx[1,2]"),
            Ok(vec![
                HostSpec {
                    numeric: 1..=1,
                    prefix: "xxx".into(),
                    num_width_chars: Some(1),
                },
                HostSpec {
                    numeric: 2..=2,
                    prefix: "xxx".into(),
                    num_width_chars: Some(1),
                }
            ])
        );
        Ok(())
    }

    #[test]
    fn test_zero() -> anyhow::Result<()> {
        assert_eq!(
            transform_single_hostspec("xxx[0123]"),
            Ok(vec![HostSpec {
                numeric: 123..=123,
                prefix: "xxx".into(),
                num_width_chars: Some(4),
            }])
        );
        Ok(())
    }

    #[test]
    fn test_tostr() -> anyhow::Result<()> {
        let demo_hs = HostSpec {
            numeric: 1..=3,
            prefix: "host".into(),
            num_width_chars: None,
        };
        let test_str = demo_hs.into_iter().fold(String::new(), |i, x| i + &x);
        assert_eq!(test_str, "host1host2host3");
        Ok(())
    }

    #[test]
    fn test_tostr_pad() -> anyhow::Result<()> {
        let demo_hs = HostSpec {
            numeric: 1..=3,
            prefix: "host".into(),
            num_width_chars: Some(3),
        };
        let test_str = demo_hs.into_iter().fold(String::new(), |i, x| i + &x);
        assert_eq!(test_str, "host001host002host003");
        Ok(())
    }
}
