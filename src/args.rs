use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::{
    audio::sample,
    wav::{self, ReadConfig},
};

#[derive(Parser, Debug, PartialEq)]
#[command(name = "wavalyze")]
#[command(about = "A audio file viewer", long_about = None)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Args {
    /// Set the log level.
    /// Examples: "error", "warn", "info", "debug", "trace"
    /// Can also be a more complex filter, e.g., "wavalyze=debug,eframe=info"
    #[arg(long, global = true)]
    pub log_level: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Files to open with optional channel and range specifications.
    /// Format: FILE[:CH_IXS][:RANGE]
    /// Examples:
    ///   song.wav
    ///   song.wav:0,2
    ///   song.wav:1000-5000
    ///   song.wav:0,2:1000-5000
    #[arg(
        value_parser = clap::value_parser!(ReadConfig),
        default_value = None
    )]
    pub files: Vec<ReadConfig>,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum Commands {
    /// Open one or more WAV files for editing
    Open {
        /// Files to open with optional channel and range specifications
        #[arg(value_parser = clap::value_parser!(ReadConfig))]
        files: Vec<ReadConfig>,
    },
    /// Compare two WAV files
    Diff {
        /// First file to compare
        #[arg(value_parser = clap::value_parser!(ReadConfig))]
        file1: ReadConfig,

        /// Second file to compare
        #[arg(value_parser = clap::value_parser!(ReadConfig))]
        file2: ReadConfig,
    },
}

fn parse_sample_ix_range(s: &str) -> Result<sample::OptIxRange> {
    let parse_bound = |s: &str| -> Result<Option<sample::Ix>> {
        if s.is_empty() {
            Ok(None)
        } else {
            // s.parse().map_err(|e| anyhow!("Invalid sample index: {e}"))
            Ok(Some(s.parse().map_err(|e| anyhow!("Invalid sample index: {e}"))?))
        }
    };

    let mut parts = s.split('-');
    let start = parse_bound(parts.next().unwrap_or(""))?;
    let end = parse_bound(parts.next().unwrap_or(""))?;

    if parts.next().is_some() {
        anyhow::bail!("Invalid sample range specification: {s}");
    }

    if let (Some(start), Some(end)) = (start, end)
        && start >= end
    {
        anyhow::bail!("Sample range start must be less than end");
    }

    Ok(sample::OptIxRange { start, end })
}

fn parse_channel_ixs(s: &str) -> Result<Vec<wav::ChIx>> {
    let channels = s
        .split(',')
        .map(|c| c.trim().parse::<wav::ChIx>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow!("Invalid channel number: {}", e))?;

    if channels.is_empty() {
        return Err(anyhow!("No channels specified"));
    }

    Ok(channels)
}

impl std::str::FromStr for ReadConfig {
    type Err = anyhow::Error;

    fn from_str(file_arg_str: &str) -> Result<Self, Self::Err> {
        let mut parts = file_arg_str.split(':');
        let filepath = PathBuf::from(parts.next().ok_or_else(|| anyhow!("Missing filepath"))?);

        let mut ch_ixs = None;
        let mut sample_range = sample::OptIxRange::default();

        for part in parts {
            if part.is_empty() {
                continue;
            }

            // Try to parse as range first, as it's more specific (contains '-')
            if part.contains('-') {
                if sample_range.start.is_some() || sample_range.end.is_some() {
                    return Err(anyhow!("Range specified multiple times"));
                }
                sample_range = parse_sample_ix_range(part)?;
            }
            // Otherwise, try to parse as channels
            else if part.contains(',') || part.chars().all(|c| c.is_ascii_digit()) {
                if ch_ixs.is_some() {
                    return Err(anyhow!("Channels specified multiple times"));
                }
                ch_ixs = Some(parse_channel_ixs(part)?);
            } else {
                return Err(anyhow!("Invalid specification part: {part}"));
            }
        }

        Ok(ReadConfig {
            filepath,
            ch_ixs,
            sample_range,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::audio::sample::IxRange;
    use std::path::PathBuf;

    #[test]
    fn test_parse_args_simple_file() {
        let args = Args::parse_from(["wavalyze", "song.wav"]);
        assert_eq!(
            args,
            Args {
                log_level: None,
                command: None,
                files: vec![ReadConfig {
                    filepath: PathBuf::from("song.wav"),
                    ch_ixs: None,
                    sample_range: sample::OptIxRange::default(),
                }]
            }
        );
    }

    #[test]
    fn test_parse_args_channels_only() {
        let args = Args::parse_from(["wavalyze", "song.wav:0,2,4"]);
        assert_eq!(
            args,
            Args {
                log_level: None,
                command: None,
                files: vec![ReadConfig {
                    filepath: PathBuf::from("song.wav"),
                    ch_ixs: Some(vec![0, 2, 4]),
                    sample_range: sample::OptIxRange::default(),
                }]
            }
        );
    }

    #[test]
    fn test_parse_args_range_only() {
        let args = Args::parse_from(["wavalyze", "song.wav:1000-5000"]);
        assert_eq!(
            args,
            Args {
                log_level: None,
                command: None,
                files: vec![ReadConfig {
                    filepath: PathBuf::from("song.wav"),
                    ch_ixs: None,
                    // sample_range: Some(IxRange::from(1000..5000)),
                    sample_range: sample::OptIxRange {
                        start: Some(1000),
                        end: Some(5000),
                    },
                }]
            }
        );
    }

    #[test]
    fn test_parse_args_channels_and_range() {
        let args = Args::parse_from(["wavalyze", "song.wav:0,2:1000-5000"]);
        assert_eq!(
            args,
            Args {
                log_level: None,
                command: None,
                files: vec![ReadConfig {
                    filepath: PathBuf::from("song.wav"),
                    ch_ixs: Some(vec![0, 2]),
                    // sample_range: Some(IxRange::from(1000..5000)),
                    sample_range: sample::OptIxRange {
                        start: Some(1000),
                        end: Some(5000),
                    },
                }]
            }
        );
    }

    #[test]
    fn test_parse_args_range_and_channels_reversed() {
        let args = Args::parse_from(["wavalyze", "song.wav:1000-5000:0,2"]);
        assert_eq!(
            args,
            Args {
                log_level: None,
                command: None,
                files: vec![ReadConfig {
                    filepath: PathBuf::from("song.wav"),
                    ch_ixs: Some(vec![0, 2]),
                    // sample_range: Some(IxRange::from(1000..5000)),
                    sample_range: sample::OptIxRange {
                        start: Some(1000),
                        end: Some(5000),
                    },
                }]
            }
        );
    }

    #[test]
    fn test_parse_args_range_start_only() {
        let args = Args::parse_from(["wavalyze", "song.wav:1000-"]);
        assert_eq!(
            args,
            Args {
                log_level: None,
                command: None,
                files: vec![ReadConfig {
                    filepath: PathBuf::from("song.wav"),
                    ch_ixs: None,
                    // sample_range: Some(IxRange::from(1000..i64::MAX)),
                    sample_range: sample::OptIxRange {
                        start: Some(1000),
                        end: None,
                    },
                }]
            }
        );
    }

    #[test]
    fn test_parse_args_range_end_only() {
        let args = Args::parse_from(["wavalyze", "song.wav:-5000"]);
        assert_eq!(
            args,
            Args {
                log_level: None,
                command: None,
                files: vec![ReadConfig {
                    filepath: PathBuf::from("song.wav"),
                    ch_ixs: None,
                    // sample_range: Some(IxRange::from(0..5000)),
                    sample_range: sample::OptIxRange {
                        start: None,
                        end: Some(5000),
                    },
                }]
            }
        );
    }

    #[test]
    fn test_parse_args_single_channel() {
        let args = Args::parse_from(["wavalyze", "song.wav:2"]);
        assert_eq!(
            args,
            Args {
                log_level: None,
                command: None,
                files: vec![ReadConfig {
                    filepath: PathBuf::from("song.wav"),
                    ch_ixs: Some(vec![2]),
                    sample_range: sample::OptIxRange::default(),
                }]
            }
        );
    }

    #[test]
    fn test_parse_args_open_subcommand() {
        let args = Args::parse_from(["wavalyze", "open", "file1.wav", "file2.wav:1:100-200"]);
        assert_eq!(
            args,
            Args {
                log_level: None,
                command: Some(Commands::Open {
                    files: vec![
                        ReadConfig {
                            filepath: PathBuf::from("file1.wav"),
                            ch_ixs: None,
                            sample_range: sample::OptIxRange::default(),
                        },
                        ReadConfig {
                            filepath: PathBuf::from("file2.wav"),
                            ch_ixs: Some(vec![1]),
                            sample_range: sample::OptIxRange {
                                start: Some(100),
                                end: Some(200),
                            },
                        }
                    ],
                }),
                files: vec![]
            }
        );
    }

    #[test]
    fn test_parse_args_diff_subcommand() {
        let args = Args::parse_from(["wavalyze", "diff", "file1.wav:0", "file2.wav:1"]);
        assert_eq!(
            args,
            Args {
                log_level: None,
                command: Some(Commands::Diff {
                    file1: ReadConfig {
                        filepath: PathBuf::from("file1.wav"),
                        ch_ixs: Some(vec![0]),
                        sample_range: sample::OptIxRange::default(),
                    },
                    file2: ReadConfig {
                        filepath: PathBuf::from("file2.wav"),
                        ch_ixs: Some(vec![1]),
                        sample_range: sample::OptIxRange::default(),
                    },
                }),
                files: vec![]
            }
        );
    }

    #[test]
    fn test_parse_invalid_range_reversed() {
        let result = Args::try_parse_from(["wavalyze", "song.wav:5000-1000"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_duplicate_channel_specs() {
        let result = Args::try_parse_from(["wavalyze", "song.wav:0,2:0,4"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_duplicate_range_specs() {
        let result = Args::try_parse_from(["wavalyze", "song.wav:100-200:300-400"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_log_level_flag() {
        let args = Args::parse_from(["wavalyze", "--log-level", "debug", "song.wav"]);
        assert_eq!(
            args,
            Args {
                log_level: Some("debug".to_string()),
                command: None,
                files: vec![ReadConfig {
                    filepath: PathBuf::from("song.wav"),
                    ch_ixs: None,
                    sample_range: sample::OptIxRange::default(),
                }]
            }
        );
    }
}
