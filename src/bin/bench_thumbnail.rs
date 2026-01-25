use wavalyze::audio::manager::AudioManager;

fn main() {
    // let args2 = wavalyze::args::Args::parse();
    // let _ = init_tracing(Some("trace"));
    println!("Hello, world!");
    let start = std::time::Instant::now();
    let mut audio = AudioManager::default();
    // let read_config = wavalyze::wav::read::ReadConfig::new("/home/emile/repos/rust/wavalyze/data/Falling_synth_and_ambience_weird_5_1.wav") .with_ch_ixs([0]);

    let read_config = wavalyze::wav::read::ReadConfig::new(
        "/home/emile/aws/Content/_verified202303/AlainClark/LetSomeAirIn/Auro10_1/48k/Original/AlainClark_LetSomeAirIn-Auro10_1-48k.wav",
    );
    let _ = audio.load_file(&read_config);
    println!("Elapsed time: {:?}", start.elapsed());
}
