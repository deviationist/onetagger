use std::time::Duration;
use anyhow::Error;
use lofty::file::AudioFile;
use lofty::probe::Probe;
use lofty::config::ParseOptions;
use std::path::{PathBuf, Path};
use std::io::BufReader;
use std::fs::File;
use rodio::{Source, Decoder};

use crate::AudioSource;

pub struct OGGSource {
    path: PathBuf,
    duration: Duration,
}

impl OGGSource {
    pub fn new(path: impl AsRef<Path>) -> Result<OGGSource, Error> {
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

        Ok(OGGSource {
            duration,
            path: path.as_ref().into()
        })
    }
}

impl AudioSource for OGGSource {
    fn duration(&self) -> u128 {
        self.duration.as_millis()
    }

    fn get_source(&self) -> Result<Box<dyn Source<Item = f32> + Send>, Error> {
        // Use rodio vorbis
        Ok(Box::new(Decoder::new_vorbis(BufReader::new(File::open(&self.path)?))?))
    }
}