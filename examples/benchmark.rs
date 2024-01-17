use std::{
    env,
    fs::File,
    path::Path,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};

fn main() -> Result<()> {
    for path in env::args().skip(1) {
        benchmark(&path, 48000, 2)?;
        benchmark(&path, 48000, 1)?;
    }
    Ok(())
}

fn benchmark<P: AsRef<Path>>(path: P, sample_rate: u32, channel_count: usize) -> Result<()> {
    let path = path.as_ref();
    let filename = path
        .file_name()
        .ok_or_else(|| anyhow!("Path is missing filename: {:?}", path))?
        .to_string_lossy();

    let mut decoder_buffer = vec![0u8; opus_embedded::Decoder::required_buffer_size(channel_count)];
    let mut sample_buffer = vec![0i16; sample_rate as usize * 120 / 1000 * channel_count];

    let start = Instant::now();
    let mut num_samples = 0;
    while start.elapsed() < Duration::from_millis(500) {
        let mut reader = ogg::PacketReader::new(File::open(path)?);
        _ = reader.read_packet_expected()?;
        _ = reader.read_packet_expected()?;

        let mut decoder =
            opus_embedded::Decoder::new(&mut decoder_buffer, sample_rate, channel_count)
                .map_err(|e| anyhow!("Failed to create Opus decoder: {}", e))?;

        while let Some(packet) = reader.read_packet()? {
            num_samples += decoder
                .decode(Some(&packet.data), &mut sample_buffer, false)
                .map_err(|e| anyhow!("Failed to decode Opus packet: {}", e))?;
        }
    }
    let elapsed = start.elapsed();
    println!(
        "{:30} {}kHz {}ch: {:.2}x",
        filename,
        sample_rate / 1000,
        channel_count,
        num_samples as f32 / sample_rate as f32 / elapsed.as_secs_f32()
    );

    Ok(())
}
