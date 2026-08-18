use std::path::{Path, PathBuf};
use anyhow::Error;
use lofty::file::AudioFile;
use lofty::probe::Probe;
use lofty::config::ParseOptions;
use std::io::BufReader;
use std::fs::File;
use rodio::{Source, Decoder};
use crate::AudioSource;

pub struct MP3Source {
    path: PathBuf,
    duration: u128
}
impl MP3Source {
    pub fn new(path: impl AsRef<Path>) -> Result<MP3Source, Error> {
        // Get duration
        // Properties only -- read_tags(false). We want the duration and nothing
        // else, and lofty's default reads tags too, so a single malformed frame
        // (a date written DD-MM-YYYY rather than the ID3v2.4 yyyy-MM-dd, say)
        // would fail the whole load and make the track unplayable. Skipping the
        // tag parse removes that entire class of failure from the audio path.
        let file = Probe::open(&path)?
            .options(ParseOptions::new().read_tags(false))
            .read()?;
        let duration = file.properties().duration();

        Ok(MP3Source {
            path: path.as_ref().to_owned(),
            duration: duration.as_millis()
        })
    }
}

impl AudioSource for MP3Source {
    // Get duration
    fn duration(&self) -> u128 {
        self.duration
    }

    // Get rodio decoder
    fn get_source(&self) -> Result<Box<dyn Source<Item = f32> + Send>, Error> {
        Ok(Box::new(Decoder::new_mp3(BufReader::new(File::open(&self.path)?))?))
    }
}