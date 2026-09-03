use anyhow::Error;
use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;
use regex::Regex;
use chrono::NaiveDate;
use reqwest::StatusCode;
use reqwest::blocking::{Client, Response};
use serde_json::Value;
use serde::{Serialize, Deserialize};
use onetagger_tag::FrameName;
use onetagger_tagger::{Track, AutotaggerSource, TaggerConfig, AudioFileInfo, MatchingUtils, TrackNumber, 
    AutotaggerSourceBuilder, PlatformInfo, PlatformCustomOptions, PlatformCustomOptionValue, supported_tags, SupportedTag, TrackMatch,
    ReleaseCandidate};

pub struct Discogs {
    client: Client,
    token: Option<String>,
    // Requests per minute
    pub rate_limit: i16,
    last_request: u128,
    // Cache of ID:Value
    release_cache: HashMap<i64, ReleaseMaster>
}

impl Discogs {
    // Create new instance
    pub fn new() -> Discogs {
        let client = Client::builder()
            .user_agent("OneTagger/1.0")
            .build()
            .unwrap();
        Discogs {
            client,
            token: None,
            rate_limit: 25, 
            last_request: 0,
            release_cache: HashMap::new()
        }
    }

    /// Set rate limit, -1 for no rate limit
    pub fn set_rate_limit(&mut self, rate_limit: i16) {
        self.rate_limit = rate_limit;
    }

    // Set authorization token and update rate limit
    pub fn set_auth_token(&mut self, token: &str) {
        self.token = Some(token.to_string());
        self.rate_limit = 60;
    }

    // Check if token is valid
    pub fn validate_token(&mut self) -> bool {
        match self.get("https://api.discogs.com/database/search", vec![("q", "test")]) {
            Ok(res) => if res.status() == StatusCode::OK { true } else { 
                error!("Failed validating Discogs token: {}", res.status());
                debug!("{:?}", res.text());
                false 
            },
            Err(e) => {
                error!("Failed validating Discogs token: {}", e);
                false
            }
        }
    }

    // Get request wrapper with rate limit
    fn get(&mut self, url: &str, query: Vec<(&str, &str)>) -> Result<Response, Error> {
        debug!("Discogs GET {}", url);
        // Rate limit
        if self.last_request > 0 && self.rate_limit != -1 {
            let diff = timestamp!() - self.last_request;
            let req_ms = 1000_f64 / (self.rate_limit as f64 / 60_f64);
            let wait = diff as f64 - req_ms;
            if wait < 0_f64 {
                debug!("Discogs rate limit delay: {}", -wait);
                sleep(Duration::from_millis(-wait as u64));
            }
        }
        // Create request
        let mut request = self.client.get(url).query(&query);
        if self.token.is_some() {
            request = request.header("Authorization", format!("Discogs token={}", self.token.as_ref().unwrap()));
        }
        let response = request.send()?;
        
        // Rate limit
        let rate_limit_remaining = response.headers().get("X-Discogs-Ratelimit-Remaining")
            .map(|v| v.to_str().unwrap().parse::<i32>().ok()).flatten().unwrap_or(100);
        debug!("Discogs rate limit remaining: {}", rate_limit_remaining);
        if rate_limit_remaining < 1 {
            warn!("Discogs rate limit hit! Waiting 10s...");
            sleep(Duration::from_secs(10));
            return self.get(url, query);
        }

        // Save last reqeust time for rate limitting
        self.last_request = timestamp!();
        Ok(response)
    }

    pub fn search(&mut self, result_type: Option<&str>, query: Option<&str>, title: Option<&str>, artist: Option<&str>) -> Result<Vec<ReleaseMasterSearchResult>, Error> {
        // Generate parameters
        let mut qp = vec![];
        if let Some(t) = result_type {
            qp.push(("type", t));
        }
        if let Some(q) = query {
            qp.push(("q", q));
        }
        if let Some(t) = title {
            qp.push(("title", t));
        }
        if let Some(a) = artist {
            qp.push(("artist", a));
        }
        // Get
        let response: Value = self.get("https://api.discogs.com/database/search", qp)?.json()?;
        let empty: Vec<Value> = vec![];
        let results: Vec<ReleaseMasterSearchResult> = response["results"].as_array().unwrap_or(&empty).into_iter().filter_map(|r| {
            // Filter only releases and masters
            let t = r["type"].as_str().unwrap_or("");
            if t == "release" || t == "master" {
                serde_json::from_value(r.to_owned()).ok()
            } else {
                None
            }
        }).collect();
        Ok(results)
    }

    // Get full release info
    pub fn full_release(&mut self, release_type: ReleaseType, id: i64) -> Result<ReleaseMaster, Error> {
        // Check if cached
        if self.release_cache.contains_key(&id) {
            return Ok(self.release_cache.get(&id).unwrap().to_owned());
        }
        // Get 
        let rtype = match release_type {
            ReleaseType::Master => "masters",
            ReleaseType::Release => "releases"
        };
        let response = self.get(&format!("https://api.discogs.com/{}/{}", rtype, id), vec![])?.json()?;
        let release: ReleaseMaster = serde_json::from_value(response)?;
        // Cache
        self.release_cache.insert(id, release.clone());
        Ok(release)
    }
}

impl AutotaggerSource for Discogs {
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Vec<TrackMatch>, Error> {
        let discogs_config: DiscogsConfig = config.get_custom("discogs")?;
        // Exact ID match
        if config.match_by_id {
            if let Some(id) = info.tags.get("DISCOGS_RELEASE_ID").map(|t| t.first().map(|id| id.trim().replace("\0", "").parse().ok()).flatten()).flatten() {
                let release = self.full_release(ReleaseType::Release, id)?;
                // Exact track number match
                if let Some(track_number) = info.track_number {
                    if track_number as usize <= release.tracks.len() {
                        let track = release.get_track(track_number as usize - 1, &discogs_config);
                        return Ok(vec![TrackMatch::new_id(track)]);
                    } else {
                        warn!("Track number out of bounds, searching normally...");
                    }
                }
                // Match inside release
                let mut tracks = vec![];
                for i in 0..release.tracks.len() {
                    tracks.push(release.get_track(i, &discogs_config));
                }
                let tracks = MatchingUtils::match_track(&info, &tracks, &config, false);
                if !tracks.is_empty() {
                    return Ok(tracks);
                }
                warn!("Falling back to normal search...");
            }
        }
        
        // Search
        let query = format!(
            "{} {}", 
            MatchingUtils::clean_title(info.title()?), 
            MatchingUtils::clean_artist_searching(info.artist()?)
        );
        debug!("Discogs query: {query}");
        let mut results = self.search(Some("release,master"), Some(&query), None, None)?;
        // Fallback
        if results.is_empty() {
            info!("Discogs fallback search!");
            results = self.search(Some("release,master"), None, Some(&MatchingUtils::clean_title(info.title()?)), Some(&info.artists.first().unwrap()))?;
        }
        if results.is_empty() {
            return Ok(vec![]);
        }
        // Turncate
        results.truncate(discogs_config.max_albums as usize);
        for release_data in results {
            // Get full
            let r = self.full_release(release_data.rtype, release_data.id);
            if r.is_err() {
                error!("{:?}", r);
                continue;
            }
            let release = r.unwrap();
            
            let mut tracks = vec![];
            for i in 0..release.tracks.len() {
                tracks.push(release.get_track(i, &discogs_config));
            }
            let tracks = MatchingUtils::match_track(&info, &tracks, &config, false);
            return Ok(tracks);
        }
        Ok(vec![])
    }

    fn extend_track(&mut self, track: &mut Track, config: &TaggerConfig) -> Result<(), Error> {
        // Check if can be extended with master
        if let Some((i, (_, v))) = track.other.iter().enumerate().find(|(_, (f, _))| f == &FrameName::same("DISCOGS_MAIN_RELEASE")) {
            // Remove temporary field
            let v = v.first().map(String::from).unwrap_or_default();
            track.other.remove(i);
            let id: i64 = match v.parse() {
                Ok(id) => id,
                Err(_) => return Ok(())
            };

            // Should be extended
            if (config.tag_enabled(SupportedTag::CatalogNumber) && track.catalog_number.is_none()) || (config.tag_enabled(SupportedTag::Label) && track.label.is_none()) {
                debug!("Discogs fetching release for catalog number/label...");
                let r = self.full_release(ReleaseType::Release, id)?;
              
                if let Some(labels) = r.labels {
                    if let Some(label) = labels.first() {
                        if track.label.is_none() {
                            track.label = Some(label.name.to_string());
                        }
                        if let Some(cn) = label.catno.as_ref() {
                            if cn != "none" {
                                track.catalog_number = Some(cn.to_owned());
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn search_releases(&mut self, query: &str, _config: &TaggerConfig) -> Result<Vec<ReleaseCandidate>, Error> {
        // Releases only, not masters. A master is a grouping: no catalogue
        // number, no country, no release date, and a tracklist borrowed from its
        // main release -- so as a tagging target it is strictly worse than the
        // pressing underneath it, and listing both would double every row.
        // Master *URLs* are still accepted; see `fetch_release`.
        Ok(self.search(Some("release"), Some(query), None, None)?
            .into_iter()
            .map(|r| r.into_candidate())
            .collect())
    }

    fn fetch_release(&mut self, id: &str, config: &TaggerConfig) -> Result<Vec<TrackMatch>, Error> {
        let discogs_config: DiscogsConfig = config.get_custom("discogs")?;
        // Bare ids are read as releases so a hand-typed number still works.
        let (kind, raw) = id.split_once(':').unwrap_or(("release", id));
        let numeric: i64 = raw.trim().parse().map_err(|_| anyhow!("Not a Discogs id: {id}"))?;

        let release = match kind {
            "master" => {
                let master = self.full_release(ReleaseType::Master, numeric)?;
                match master.main_release {
                    // Resolve to the pressing Discogs itself calls canonical --
                    // that is the one carrying label, catalogue number and date.
                    Some(main) => self.full_release(ReleaseType::Release, main)?,
                    // Malformed, but the master's own tracklist still tags.
                    None => master,
                }
            },
            _ => self.full_release(ReleaseType::Release, numeric)?,
        };

        Ok(release.tracks
            .iter()
            .enumerate()
            // Discogs tracklists interleave headings ("Side A") with tracks, and
            // a heading has no position. Enumerated *before* filtering because
            // `get_track` indexes into the untouched list.
            .filter(|(_, t)| !t.position.trim().is_empty())
            .map(|(i, _)| TrackMatch::new_id(release.get_track(i, &discogs_config)))
            .collect())
    }

}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseType {
    Release,
    Master
}

// Used in search results
///
/// Everything past `url` was commented out upstream, which is fine while the
/// only consumer is `match_track` -- it fetches the full release anyway and
/// needs nothing but the id. A direct-match picker is the opposite case: the
/// operator chooses *between* these rows, and two pressings of one album are
/// told apart by year, country, format and catalogue number alone.
///
/// All optional, and none of it trusted: Discogs omits fields per result type
/// (a master has no catno or country) and `year` arrives as a string.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseMasterSearchResult {
    pub id: i64,
    #[serde(rename = "type")]
    pub rtype: ReleaseType,
    #[serde(rename = "resource_url")]
    pub url: String,
    /// "Artist - Album", already joined by Discogs. Left whole rather than split
    /// on " - ": plenty of artist names contain that separator themselves.
    pub title: Option<String>,
    pub year: Option<String>,
    pub country: Option<String>,
    pub label: Option<Vec<String>>,
    pub format: Option<Vec<String>>,
    pub catno: Option<String>,
    pub thumb: Option<String>,
    pub cover_image: Option<String>,
    pub uri: Option<String>,
}

impl ReleaseMasterSearchResult {
    /// Into a platform-neutral picker row
    pub fn into_candidate(self) -> ReleaseCandidate {
        ReleaseCandidate {
            id: format!("{}:{}", match self.rtype {
                ReleaseType::Release => "release",
                ReleaseType::Master => "master",
            }, self.id),
            title: self.title.unwrap_or_else(|| format!("Discogs {}", self.id)),
            // Search rows carry no artist array of their own -- the name is
            // inside `title`. Left empty rather than guessed at.
            artists: vec![],
            year: self.year.as_ref().and_then(|y| y.trim().parse().ok()),
            label: self.label.as_ref().and_then(|l| l.first().cloned()),
            catalog_number: self.catno.filter(|c| c != "none" && !c.trim().is_empty()),
            format: self.format.as_ref().map(|f| f.join(", ")).filter(|f| !f.is_empty()),
            country: self.country,
            // Not in a search response; only a full fetch knows.
            track_count: None,
            art: self.thumb.filter(|t| !t.is_empty()).or(self.cover_image),
            url: Some(match self.uri {
                Some(uri) if uri.starts_with("http") => uri,
                Some(uri) => format!("https://www.discogs.com{uri}"),
                None => format!("https://www.discogs.com/release/{}", self.id),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artist {
    pub name: String,
    pub id: i64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtraArtist {
    pub name: String,
    pub id: i64,
    pub role: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscogsTrack {
    pub position: String,
    pub title: String,
    pub artists: Option<Vec<ExtraArtist>>,
    pub duration: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: i64,
    pub name: String,
    pub catno: Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub height: i64,
    pub width: i64,
    #[serde(rename = "uri")]
    pub url: String,
    #[serde(rename = "type")]
    pub image_type: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseFormat {
    pub name: String,
    pub qty: String,
    pub descriptions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseMaster {
    pub id: i64,
    pub styles: Option<Vec<String>>,
    pub genres: Vec<String>,
    pub year: Option<i16>,
    pub artists: Vec<Artist>,
    #[serde(rename = "extraartists")]
    pub extra_artists: Option<Vec<ExtraArtist>>,
    pub country: Option<String>,
    #[serde(rename = "uri")]
    pub url: String,
    pub labels: Option<Vec<Label>>,
    pub title: String,
    pub images: Option<Vec<Image>>,
    #[serde(rename = "tracklist")]
    pub tracks: Vec<DiscogsTrack>,
    pub released: Option<String>,
    pub main_release: Option<i64>,
    pub formats: Option<Vec<ReleaseFormat>>
}

impl ReleaseMaster {
    // Remove (n) at end of artist
    pub fn clean_artist(input: &str) -> String {
        let re = Regex::new(r" \(\d{1,2}\)$").unwrap();
        re.replace(input, "").to_string()
    }

    pub fn get_track(&self, track_index: usize, discogs_config: &DiscogsConfig) -> Track {
        // Parse release date
        let release_date = match &self.released {
            Some(r) => NaiveDate::parse_from_str(&r, "%Y-%m-%d").ok(),
            None => None
        };

        // Get catalog number
        let mut catalog_number = None;
        if let Some(labels) = &self.labels {
            if let Some(label) = labels.first() {
                if let Some(cn) = label.catno.as_ref() {
                    if cn != "none" {
                        catalog_number = Some(cn.to_string());
                    }
                }
            }
        }

        // Parse track number
        let mut track_number = TrackNumber::Number((track_index + 1) as i32);
        let mut disc_number = None;
        let position = self.tracks[track_index].position.to_string();

        if discogs_config.track_number_int {
            let re = Regex::new("(\\d+)(\\.|-)(\\d+)").unwrap();
            if let Some(captures) = re.captures(&position) {
                disc_number = Some(captures.get(1).unwrap().as_str().parse::<u16>().ok()).flatten();
                track_number = TrackNumber::Number(captures.get(3).unwrap().as_str().parse().unwrap());
            } 
        } else {
            track_number = TrackNumber::Custom(position);
        }

        // Other frames
        let mut other = vec![
            (FrameName::same("VINYLTRACK"), vec![self.tracks[track_index].position.to_string()])
        ];
        if let Some(formats) = &self.formats {
            let formats = formats.iter().map(|f| match f.descriptions.as_ref() {
                Some(description) => format!("{} x {}, {}", f.qty, f.name, description.join(", ")),
                None => format!("{} x {}", f.qty, f.name),
            }).collect::<Vec<_>>();
            other.push((FrameName::same("MEDIATYPE"), formats));
        }

        // Add main release tag which is used by extend_track
        if let Some(main_release) = self.main_release {
            other.push((FrameName::same("DISCOGS_MAIN_RELEASE"), vec![main_release.to_string()]));
        }

        // Generate track
        Track {
            platform: "discogs".to_string(),
            title: self.tracks[track_index].title.to_string(),
            artists: match self.tracks[track_index].artists.as_ref() {
                // Use track artists if available
                Some(artists) => artists.iter().map(|a| ReleaseMaster::clean_artist(&a.name).to_string()).collect(),
                None => self.artists.iter().map(|a| ReleaseMaster::clean_artist(&a.name).to_string()).collect()
            },
            album_artists: self.artists.iter().map(|a| ReleaseMaster::clean_artist(&a.name).to_string()).collect(),
            album: Some(self.title.to_string()),
            genres: self.genres.clone(),
            styles: self.styles.clone().unwrap_or(vec![]),
            art: match self.images.as_ref().unwrap_or(&Vec::new()).first() {
                Some(image) => Some(image.url.to_string()),
                None => None
            },
            url: self.url.to_string(),
            label: if let Some(labels) = &self.labels {
                if labels.is_empty() {
                    None
                } else {
                    Some(ReleaseMaster::clean_artist(&labels[0].name).to_string())
                }
            } else {None},
            release_year: self.year,
            release_date,
            catalog_number,
            track_id: None,
            release_id: Some(self.id.to_string()),
            duration: MatchingUtils::parse_duration(&self.tracks[track_index].duration).unwrap_or(Duration::ZERO).into(),
            track_number: Some(track_number),
            disc_number,
            track_total: Some(self.tracks.len() as u16),
            other,
            thumbnail: self.images.as_ref().unwrap_or(&Vec::new()).iter().min_by(|a, b| a.width.cmp(&b.width)).map(|i| i.url.to_string()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiscogsBuilder {}

impl AutotaggerSourceBuilder for DiscogsBuilder {
    fn new() -> DiscogsBuilder {
        DiscogsBuilder {}
    }

    fn get_source(&mut self, config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Error> {
        let config: DiscogsConfig = config.get_custom("discogs")?;
        let mut discogs = Discogs::new();
        // Auth
        discogs.set_auth_token(&config.token);
        if !discogs.validate_token() {
            return Err(anyhow!("Invalid Discogs token!"));
        }
        if let Some(rl) = config.rate_limit {
            discogs.set_rate_limit(rl as i16);
        }
        Ok(Box::new(discogs))
    }

    fn supports_direct_match(&self) -> bool {
        true
    }

    /// Recognise a Discogs release or master URL.
    ///
    /// Covers the shapes a browser actually produces: a bare
    /// `discogs.com/release/12345`, the slugged `.../release/12345-Some-Title`,
    /// the localised `.../fr/release/12345`, and the API's plural
    /// `api.discogs.com/releases/12345`. The id is returned prefixed, because
    /// 12345 names two different records depending on which of the two it is.
    fn parse_url(&self, url: &str) -> Option<String> {
        let re = Regex::new(r"(?i)discogs\.com/(?:[a-z]{2}(?:_[A-Z]{2})?/)?(release|master)s?/(\d+)").ok()?;
        let c = re.captures(url)?;
        Some(format!("{}:{}", c.get(1)?.as_str().to_lowercase(), c.get(2)?.as_str()))
    }

    fn info(&self) -> PlatformInfo {
        PlatformInfo {
            id: "discogs".to_string(),
            name: "Discogs".to_string(),
            description: "Slow due to rate limits (~25 tracks / min) & requires a free account".to_string(),
            icon: include_bytes!("../assets/discogs.png"),
            max_threads: 1,
            version: "1.0.0".to_string(),
            requires_auth: true,
            supported_tags: supported_tags!(Title, Artist, AlbumArtist, Album, Genre, Style, AlbumArt, URL, Label, ReleaseDate, CatalogNumber, ReleaseId, Duration, TrackNumber, DiscNumber, TrackTotal, OtherTags),
            custom_options: PlatformCustomOptions::new()
                // Discogs token
                .add_tooltip("token", "Token", 
                "To obtain token, create a free account on Discogs. More info? Click <q-icon style='padding-bottom: 4px;' name='mdi-help-circle-outline'></q-icon> HELP on the right", 
                PlatformCustomOptionValue::String { value: String::new(), hidden: Some(false) })
                // How many max albums to check
                .add_tooltip("max_albums", "Max albums to check", 
                "How many albums in search results to check. Due to rate limiting this increases tagging time by a lot", 
                PlatformCustomOptionValue::Number { min: 1, max: 16, step: 1, value: 4 })
                // Write track number as int
                .add("track_number_int", "Write track number as number, rather than Discogs's format", PlatformCustomOptionValue::Boolean { value: false })
        }
    }
}

#[derive(Deserialize)]
pub struct DiscogsConfig {
    pub token: String,
    pub max_albums: i32,
    pub track_number_int: bool,
    pub rate_limit: Option<i32>,
}
