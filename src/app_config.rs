use clap::Parser;
use std::str::FromStr;

use crate::wav::read::ChIx;

/// A file to be used in a diff operation, with optional channel and offset
#[derive(Debug, Clone, PartialEq)]
pub struct InputAudioFile {
    pub path: String,
    pub channel: Option<ChIx>,
    pub offset: Option<u32>,
}

impl FromStr for InputAudioFile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.is_empty() {
            return Err("Empty string".to_string());
        }

        let mut path_parts: Vec<&str> = Vec::new();
        let mut channel = None;
        let mut offset = None;

        let mut options_started = false;
        for part in parts.iter().rev() {
            if options_started {
                path_parts.insert(0, part);
                continue;
            }

            let mut kv = part.splitn(2, '=');
            if let (Some(key), Some(value)) = (kv.next(), kv.next()) {
                match key {
                    "channel" => {
                        if channel.is_some() {
                            return Err("channel specified more than once".to_string());
                        }
                        channel = Some(value.parse::<usize>().map_err(|e| e.to_string())?);
                    }
                    "offset" => {
                        if offset.is_some() {
                            return Err("offset specified more than once".to_string());
                        }
                        offset = Some(value.parse::<u32>().map_err(|e| e.to_string())?);
                    }
                    _ => {
                        // Not a key we know, assume it's part of the path
                        // options_started = true;
                        // path_parts.insert(0, part);
                        return Err(format!("Unknown option: {}", part));
                    }
                }
            } else {
                // Not a key-value pair, assume it's part of the path
                options_started = true;
                path_parts.insert(0, part);
            }
        }

        if path_parts.is_empty() {
            return Err("No path specified".to_string());
        }

        Ok(InputAudioFile {
            path: path_parts.join(":"),
            channel,
            offset,
        })
    }
}

/// Application configuration, parsed from command line arguments
#[derive(Parser, Debug, Default)]
#[command(version, about, long_about = None)]
pub struct AppConfig {
    /// Audio files to open
    pub audio_files: Vec<String>,

    /// Optional start sample, if not set, defaults to 0
    #[clap(short, long)]
    pub start: Option<u32>,

    /// Optional end sample, if not set, defaults to last sample
    #[clap(short, long)]
    pub end: Option<u32>,

    /// Two audio files to diff
    #[clap(short, long, num_args = 2, value_names = ["FILE1", "FILE2"])]
    pub diff: Option<Vec<InputAudioFile>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config() {
        println!("test");

        let args = AppConfig::try_parse_from(["test", "--start", "1", "--end", "2", "file1.wav", "file2.wav"]);
        println!("{:?}", args);
        assert!(args.is_ok());
        if let Ok(args) = args {
            assert_eq!(args.audio_files.len(), 2);
            assert_eq!(args.audio_files[0], "file1.wav");
            assert_eq!(args.audio_files[1], "file2.wav");
            assert_eq!(args.start, Some(1));
            assert_eq!(args.end, Some(2));
            assert!(args.diff.is_none());
        }

        let args = AppConfig::try_parse_from(["test", "--end", "2", "file1.wav", "file2.wav"]);
        println!("{:?}", args);
        assert!(args.is_ok());
        if let Ok(args) = args {
            assert_eq!(args.audio_files.len(), 2);
            assert_eq!(args.start, None);
            assert_eq!(args.end, Some(2));
            assert!(args.diff.is_none());
        }

        let args = AppConfig::try_parse_from(["test"]);
        println!("{:?}", args);
        assert!(args.is_ok());
        if let Ok(args) = args {
            assert!(args.audio_files.is_empty());
            assert_eq!(args.start, None);
            assert_eq!(args.end, None);
            assert!(args.diff.is_none());
        }

        let args = AppConfig::try_parse_from(["test", "--diff", "file1.wav", "file2.wav"]);
        println!("{:?}", args);
        assert!(args.is_ok());
        if let Ok(args) = args {
            assert!(args.audio_files.is_empty());
            assert!(args.diff.is_some());
            let diff_files = args.diff.unwrap();
            assert_eq!(diff_files.len(), 2);
            assert_eq!(
                diff_files[0],
                InputAudioFile {
                    path: "file1.wav".to_string(),
                    channel: None,
                    offset: None
                }
            );
            assert_eq!(
                diff_files[1],
                InputAudioFile {
                    path: "file2.wav".to_string(),
                    channel: None,
                    offset: None
                }
            );
        }

        let args = AppConfig::try_parse_from(["test", "some.wav", "--diff", "file1.wav", "file2.wav"]);
        println!("{:?}", args);
        assert!(args.is_ok());
        if let Ok(args) = args {
            assert_eq!(args.audio_files.len(), 1);
            assert_eq!(args.audio_files[0], "some.wav");
            assert!(args.diff.is_some());
            let diff_files = args.diff.unwrap();
            assert_eq!(diff_files.len(), 2);
            assert_eq!(
                diff_files[0],
                InputAudioFile {
                    path: "file1.wav".to_string(),
                    channel: None,
                    offset: None
                }
            );
            assert_eq!(
                diff_files[1],
                InputAudioFile {
                    path: "file2.wav".to_string(),
                    channel: None,
                    offset: None
                }
            );
        }

        let args = AppConfig::try_parse_from(["test", "--diff", "file1.wav"]);
        assert!(args.is_err());
    }

    #[test]
    fn test_diff_file_parsing() {
        let args = AppConfig::try_parse_from(["test", "--diff", "file1.wav:offset=10", "file2.wav:channel=1:offset=20"]);
        assert!(args.is_ok());
        if let Ok(args) = args {
            assert!(args.diff.is_some());
            let diff_files = args.diff.unwrap();
            assert_eq!(diff_files.len(), 2);
            assert_eq!(
                diff_files[0],
                InputAudioFile {
                    path: "file1.wav".to_string(),
                    channel: None,
                    offset: Some(10)
                }
            );
            assert_eq!(
                diff_files[1],
                InputAudioFile {
                    path: "file2.wav".to_string(),
                    channel: Some(1),
                    offset: Some(20)
                }
            );
        }
    }
}
