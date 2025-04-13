use clap::Parser;

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
        }

        let args = AppConfig::try_parse_from(["test", "--end", "2", "file1.wav", "file2.wav"]);
        println!("{:?}", args);
        assert!(args.is_ok());
        if let Ok(args) = args {
            assert_eq!(args.audio_files.len(), 2);
            assert_eq!(args.start, None);
            assert_eq!(args.end, Some(2));
        }

        let args = AppConfig::try_parse_from(["test"]);
        println!("{:?}", args);
        assert!(args.is_ok());
        if let Ok(args) = args {
            assert!(args.audio_files.is_empty());
            assert_eq!(args.start, None);
            assert_eq!(args.end, None);
        }
    }
}
