use std::fs::File;
use std::io::BufReader;
use std::time::Duration;
use rodio::{Decoder, MixerDeviceSink, Player};
use rodio::source::{SineWave, Source};

pub fn play() {
    // _stream must live as long as the sink
    let handle = rodio::DeviceSinkBuilder::open_default_sink()
        .expect("open default audio stream");
    let player = rodio::Player::connect_new(&handle.mixer());

    let file = BufReader::new(File::open("C:\\Users\\maelb\\Desktop\\music\\mrbright.wav").unwrap());
    let source = Decoder::try_from(file).unwrap();

    player.append(source);
    // The sound plays in a separate thread. This call will block the current thread until the
    // player has finished playing all its queued sounds.
    player.sleep_until_end();
}
