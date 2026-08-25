use std::collections::HashMap;
use anyhow::Error;
use axum::extract::ws::{WebSocket, Message};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};
use onetagger_renamer::ac::Autocomplete;
use onetagger_renamer::docs::FullDocs;
use onetagger_renamer::{Renamer, TemplateParser, RenamerConfig};
use serde_json::{Value, json};
use serde::{Serialize, Deserialize};
use onetagger_tag::{TagChanges, TagSeparators, Tag, Field};
use onetagger_tagger::{TaggerConfig, AudioFileInfo, Track, TrackMatch};
use onetagger_autotag::{Tagger, AudioFileInfoImpl, TaggerConfigExt, AUTOTAGGER_PLATFORMS};
use onetagger_autotag::audiofeatures::{AudioFeaturesConfig, AudioFeatures};
use onetagger_platforms::spotify::Spotify;
use onetagger_player::{AudioSources, AudioPlayer};
use onetagger_shared::{Settings, COMMIT};
use onetagger_playlist::{UIPlaylist, PLAYLIST_EXTENSIONS, get_files_from_playlist_file};

/// How many tracks Quick Tag loads before it stops and offers "Show all".
/// The cap exists to keep the initial websocket payload small, not because
/// loading is slow: measured on a 773-file folder the text tags average
/// ~2.2KB/file, so this bound is ~2.3MB of JSON. Album art is NOT included
/// (it is fetched lazily per row), which is what makes the bound cheap.
pub const QUICKTAG_LOAD_LIMIT: usize = 1000;
/// Cap on library-search results. Matching is cheap, but every Quick Tag hit
/// costs a tag read (~90ms over NFS), so an unbounded query on a broad term
/// would stall the socket for minutes.
pub const SEARCH_LIMIT: usize = 500;

use crate::StartContext;
use crate::quicktag::{QuickTag, QuickTagFile, QuickTagData};
use crate::tageditor::TagEditor;
use crate::browser::{FileBrowser, FolderBrowser};
use crate::paths;


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
enum Action {
    Init,
    Exit,
    SaveSettings { settings: Value },
    LoadSettings,
    DefaultCustomPlatformSettings,
    Browse { path: Option<String>, context: Option<String> },
    Browser { url: String },
    OpenSettingsFolder,
    OpenFolder { path: PathBuf },
    OpenFile { path: PathBuf },
    DeleteFiles { paths: Vec<String> },
    GetLog,
    GeneratePlaylist { paths: Vec<String> },

    LoadPlatforms,
    StartTagging { config: TaggerConfigs, playlist: Option<UIPlaylist> },
    StopTagging,
    ConfigCallback { config: Value, platform: String, id: String },
    RepoManifest,
    #[serde(rename_all = "camelCase")]
    InstallPlatform { id: String, version: String, is_native: bool },

    Waveform { path: PathBuf },
    PlayerLoad { path: PathBuf },
    PlayerPlay, 
    PlayerPause,
    PlayerSeek { pos: u64 },
    PlayerVolume { volume: f32 },
    PlayerStop,

    QuickTagLoad { path: Option<String>, playlist: Option<UIPlaylist>, recursive: Option<bool>, separators: TagSeparators, limit: Option<bool> },
    QuickTagSave { changes: TagChanges },
    QuickTagFolder { path: Option<String>, subdir: Option<String> },
    /// Library-wide search by path/filename. Separate from QuickTagFolder
    /// because it returns loaded tracks, not a directory listing.
    QuickTagSearch { query: String, path: Option<String>, separators: TagSeparators, limit: Option<usize> },

    #[serde(rename_all = "camelCase")]
    SpotifyAuthorize { client_id: String, client_secret: String },
    SpotifyAuthorized,

    TagEditorFolder { path: Option<String>, subdir: Option<String>, recursive: Option<bool>  },
    /// Cheap "has this folder changed" probe, so a view can notice files
    /// appearing or disappearing without re-listing on a timer.
    FolderSignature { path: String },
    /// Library-wide search by path/filename. Returns bare entries -- the Tag
    /// Editor's result rows show path and filename only, so no tags are read.
    TagEditorSearch { query: String, path: Option<String>, limit: Option<usize> },
    TagEditorLoad { path: PathBuf },
    TagEditorSave { changes: TagChanges },

    RenamerSyntaxHighlight { template: String },
    RenamerAutocomplete { template: String },
    RenamerPreview { config: RenamerConfig },
    RenamerStart { config: RenamerConfig },

    FolderBrowser { path: PathBuf, child: String, base: bool },

    ManualTag { config: TaggerConfig, path: PathBuf },
    ManualTagApply { matches: Vec<TrackMatch>, path: PathBuf, config: TaggerConfig },

    /// Extend every match so the matrix view draws what a platform can actually
    /// offer, rather than only what its search returned. Separate from apply
    /// because the operator chooses from these values before applying anything.
    ManualTagExtend { matches: Vec<TrackMatch>, path: PathBuf, config: TaggerConfig },
    /// A compact summary of the tags the file already has, so the manual tagger
    /// can show what you are comparing the sources against.
    ManualTagFileInfo { path: PathBuf },
    /// Write a track the operator composed field by field in the matrix view.
    /// The precedence merge behind `ManualTagApply` cannot express this: it only
    /// knows first-selection-wins for scalars and union for arrays.
    ManualTagApplyComposed { track: Track, path: PathBuf, config: TaggerConfig },
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
enum TaggerConfigs {
    AutoTagger(TaggerConfig), 
    AudioFeatures(AudioFeaturesConfig)
}

impl TaggerConfigs {
    // Print to log for later easier debug
    pub fn debug_print(&self) {
        match self {
            TaggerConfigs::AutoTagger(c) => {
                let mut c = c.clone();
                // don't leak secrets
                c.custom = HashMap::new().into();
                c.spotify = None;
                info!("AutoTagger config: {:?}", c);
            },
            TaggerConfigs::AudioFeatures(c) => {
                info!("AudioFeatures Config: {:?}", c);
            }
        }
    }
}

// Shared variables in socket
struct SocketContext {
    player: AudioPlayer,
    spotify: Option<Spotify>,
    start_context: StartContext
} 

impl SocketContext {
    pub fn new(start_context: StartContext) -> SocketContext {
        SocketContext {
            player: AudioPlayer::new(),
            spotify: None,
            start_context
        }
    }
}


/// Reply to init call
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InitData {
    action: &'static str,
    version: &'static str,
    os: &'static str,
    arch: &'static str,
    custom_platform_compat: i32,
    start_context: StartContext,
    renamer_docs: FullDocs,
    commit: &'static str,
    work_dir: PathBuf,
    data_dir: PathBuf
}

impl InitData {
    /// Create new default instance
    pub fn new(start_context: StartContext) -> InitData {
        InitData {
            action: "init",
            version: onetagger_shared::VERSION,
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            custom_platform_compat: onetagger_tagger::custom::CUSTOM_PLATFORM_COMPATIBILITY,
            start_context,
            renamer_docs: FullDocs::get().html(),
            commit: COMMIT,
            work_dir: std::env::current_dir().unwrap_or_default(),
            data_dir: Settings::get_folder().unwrap_or_default(),
        }
    }
}

pub(crate) async fn handle_ws_connection(mut websocket: WebSocket, context: StartContext) -> Result<(), Error> {
    let mut context = SocketContext::new(context);
    
    while let Some(message) = websocket.recv().await {
        match message {
            Ok(msg) => {
                match msg.to_text() {
                    // A Close frame stringifies to "", which the parser then
                    // rejects as EOF -- logging an error for an ordinary
                    // disconnect and trying to answer on a socket that is
                    // already gone. Nothing empty is ever a command.
                    Ok(text) if text.is_empty() => {},
                    Ok(text) => {
                        // Handle the WS message
                        match handle_message(text, &mut websocket, &mut context).await {
                            Ok(_) => {},
                            Err(err) => {
                                // Send error to UI
                                error!("Websocket: {:?}, Data: {}", err, text);
                                send_socket(&mut websocket, json!({
                                    "action": "error",
                                    "message": &format!("{}", err)
                                })).await.ok();
                            }
                        }
                    },
                    Err(e) => warn!("WebSocket Message is not text: {e}"),
                }
            }

            Err(e) => {
                warn!("WebSocket error: {e}");
            }
        }
    
    }

    Ok(())
}

/// Serialize and send to socket with warning intercept
async fn send_socket<D: Serialize>(ws: &mut WebSocket, json: D) -> Result<(), Error> {
    match send_socket_inner(ws, json).await {
        Ok(_) => Ok(()),
        Err(e) => {
            warn!("Failed sending to socket: {e}");
            Err(e)
        },
    }
}

/// Serialize and send to socket
async fn send_socket_inner<D: Serialize>(ws: &mut WebSocket, json: D) -> Result<(), Error> {
    ws.send(Message::from(serde_json::to_string(&json)?)).await?;
    Ok(())
}

/// Confine both ends of a renamer run.
///
/// The renamer reads from `path` and writes to `out_dir`, so an unchecked
/// out_dir would move files out of the library -- the same outcome as a delete,
/// reached by a different route. Preview shares this because it walks the source
/// tree too, and because a preview that succeeds on a path the run then refuses
/// is a worse experience than failing once.
fn confine_renamer(mut config: RenamerConfig) -> Result<RenamerConfig, Error> {
    config.path = paths::confine(&config.path)?;
    if let Some(out_dir) = config.out_dir {
        config.out_dir = Some(paths::confine(&out_dir)?);
    }
    Ok(config)
}

/// Resolve a client-supplied search root, defaulting to the library.
///
/// `unwrap_or_default()` on a missing path yields an empty PathBuf, which walks
/// the process's working directory -- somewhere the client never named and, on
/// a server, nowhere near the library.
fn search_root(path: Option<String>) -> Result<PathBuf, Error> {
    match path {
        Some(path) => paths::confine(PathBuf::from(path)),
        None => match paths::root() {
            Some(root) => Ok(root.to_path_buf()),
            None => Err(Error::msg("No search root given and no library configured"))
        }
    }
}

async fn handle_message(text: &str, websocket: &mut WebSocket, context: &mut SocketContext) -> Result<(), Error> {
    // Parse JSON
    let action: Action = serde_json::from_str(text)?;
    match action {
        // Get initial info
        Action::Init => {
            send_socket(websocket, InitData::new(context.start_context.clone())).await.ok();
        },
        Action::Exit => std::process::exit(0),
        Action::SaveSettings { settings } => Settings::from_ui(&settings).save()?,
        Action::LoadSettings => match Settings::load() {
            Ok(settings) => {
                send_socket(websocket, json!({
                    "action": "loadSettings",
                    "settings": settings.ui
                })).await.ok();
            }
            // Ignore settings if they don't exist (might be initial load)
            Err(e) => error!("Failed loading settings, using defaults. {}", e)
        },
        // Get the default custom platform options
        Action::DefaultCustomPlatformSettings => {
            send_socket(websocket, json!({
                "action": "defaultCustomPlatformSettings",
                "custom": TaggerConfig::custom_default().custom
            })).await.ok();
        }
        // Browse for folder
        Action::Browse { path, context } => {
            let mut initial = path.unwrap_or(".".to_string());
            if initial.is_empty() || !Path::new(&initial).exists() {
                initial = ".".to_string()
            }
            if let Some(path) = tinyfiledialogs::select_folder_dialog("Select path", &initial) {
                send_socket(websocket, json!({
                    "action": "browse",
                    "path": path,
                    "context": context
                })).await.ok();
            }
        },
        // Get 1t Log
        Action::GetLog => {
            log::logger().flush();
            let log = std::fs::read_to_string(&Settings::get_folder()?.join("onetagger.log"))?;
            send_socket(websocket, json!({
                "action": "log",
                "log": log
            })).await.ok();
        },
        // Open URL in external browser
        Action::Browser { url } => { webbrowser::open(&url)?; },
        Action::OpenSettingsFolder => opener::open(Settings::get_folder()?.to_str().unwrap())?,
        // Handing a path to the desktop shell is still acting on it, and on a
        // server these do nothing useful anyway.
        Action::OpenFolder { path } => { opener::open(paths::confine(&path)?).ok(); },
        Action::OpenFile { path } => { opener::open(paths::confine(&path)?).ok(); },
        // Delete outright rather than to the OS trash. Upstream trashes because a
        // desktop user has no other net; this fork targets libraries on network
        // storage that snapshots, which is a better one -- it covers overwrite and
        // corruption too, and it pins a trashed copy exactly as long as a deleted
        // one, so the trash bought clutter and no safety. Worse, the trash it
        // creates lands at the top of the volume holding the file, which for a
        // mounted library means inside the library, in a dot-directory the file
        // browser deliberately hides. Files disappeared into a folder the app that
        // made it would not show. The UI confirms before this is reached.
        Action::DeleteFiles { paths: requested } => {
            // Paths arrive from the client, so they are input, not instructions.
            // Anything reaching this socket can name any path on the host, and
            // the process would happily unlink whatever its uid can reach -- a
            // config file, another service's data, its own settings. Confine it
            // to the library the operator started the server on.
            let mut deleted = Vec::new();
            let mut failed = Vec::new();
            for path in &requested {
                // Per path rather than all-or-nothing: a selection with one bad
                // entry should delete the rest and report which failed.
                let resolved = match paths::confine(path) {
                    Ok(resolved) => resolved,
                    Err(e) => {
                        warn!("Refusing to delete {path}: {e}");
                        failed.push(format!("{e}"));
                        continue;
                    }
                };
                match std::fs::remove_file(&resolved) {
                    Ok(_) => deleted.push(path.clone()),
                    Err(e) => {
                        error!("Failed deleting {path}: {e}");
                        failed.push(format!("{path}: {e}"));
                    }
                }
            }
            // Acknowledge what actually went, not what was asked for: a partial
            // failure should drop exactly the rows that are gone, and the client
            // cannot tell which those are unless it is told.
            send_socket(websocket, json!({
                "action": "deleteFiles",
                "paths": deleted
            })).await.ok();
            if !failed.is_empty() {
                return Err(Error::msg(format!("Failed deleting: {}", failed.join("; "))));
            }
        },

        Action::GeneratePlaylist { paths: requested } => {
            // All-or-nothing here, unlike delete: a playlist quietly missing the
            // entries that failed is worse than no playlist.
            let entries = paths::confine_all(&requested)?;
            let playlist = onetagger_playlist::create_m3u_playlist(&entries);
            if let Some(path) = tinyfiledialogs::save_file_dialog_with_filter(
                "Save playlist", 
                &std::env::current_dir()?.to_string_lossy().to_string(), 
                &["m3u", "m3u8"], 
                "Save playlist"
            ) {
                std::fs::write(&path, playlist)?;
                send_socket(websocket, json!({
                    "action": "notify",
                    "message": format!("Playlist saved to: {path}")
                })).await.ok();
            }
        }

        Action::LoadPlatforms => {
            let platforms = tokio::task::spawn_blocking(|| {
                let mut platforms = AUTOTAGGER_PLATFORMS.lock().unwrap();
                platforms.load_all();
                platforms.platforms.iter().map(|p| p.info.clone()).collect::<Vec<_>>()
            }).await?;
            send_socket(websocket, json!({
                "action": "loadPlatforms",
                "platforms": platforms
            })).await.ok();
        },
        Action::ConfigCallback { config, platform, id } => {
            let platform_clone = platform.clone();
            let response = tokio::task::spawn_blocking(move || {
                if let Some(p) = AUTOTAGGER_PLATFORMS.lock().unwrap().get_builder(&platform) {
                    Some(p.config_callback(&id, config))
                } else {
                    None
                }
            }).await?;
            if let Some(r) = response {
                send_socket(websocket, json!({
                    "action": "configCallback",
                    "platform": platform_clone,
                    "response": r
                })).await.ok();
            }
        }
        Action::StartTagging { config, playlist } => {
            config.debug_print();

            // Load playlist. Its entries arrive as base64 M3U text from the
            // browser, so they are client-controlled paths like any other -- and
            // this is the tagger, which rewrites what it is given.
            let mut files = if let Some(playlist) = playlist {
                paths::confine_all(&playlist.get_files()?)?
            } else { vec![] };
            let mut file_count = files.len();
            let mut folder_path = None;
            let tagger_finished = Arc::new(Mutex::new(None));
            // Load taggers
            let (tagger_type, rx) = match config {
                TaggerConfigs::AutoTagger(c) => {
                    // Load file list
                    if files.is_empty() {
                        // The tagger rewrites every file it is handed, so the
                        // folder it walks is the highest-consequence path the
                        // client gets to name.
                        let path = paths::confine(c.path.as_ref().map(|p| p.to_owned()).unwrap_or_default())?
                            .to_string_lossy().to_string();
                        files = AudioFileInfo::get_file_list(&path, c.include_subfolders);
                        file_count = files.len();
                        folder_path = Some(path);
                    }
                    let rx = Tagger::tag_files(&c, files, tagger_finished.clone());
                    ("autoTagger", rx)
                },
                TaggerConfigs::AudioFeatures(c) => {
                    if files.is_empty() {
                        let path = paths::confine(c.path.as_ref().map(|i| i.to_owned()).unwrap_or_default())?
                            .to_string_lossy().to_string();
                        files = AudioFileInfo::get_file_list(&path, c.include_subfolders);
                        folder_path = Some(path);
                        file_count = files.len();
                    }
                    // Authorize spotify
                    let spotify = context.spotify.as_ref().ok_or(anyhow!("Spotify unauthorized!"))?.to_owned().to_owned();
                    let rx = AudioFeatures::start_tagging(c.clone(), spotify, files);
                    ("audioFeatures", rx)
                },
            };

            // Start
            let start = timestamp!();
            send_socket(websocket, json!({
                "action": "startTagging",
                "files": file_count,
                "type": tagger_type
            })).await.ok();
            // Tagging
            for status in rx {
                send_socket(websocket, json!({
                    "action": "taggingProgress",
                    "status": status
                })).await.ok();
            }
            info!("Tagging finished, took: {} seconds.", (timestamp!() - start) / 1000);
            // Done
            send_socket(websocket, json!({
                "action": "taggingDone",
                "path": folder_path,
                "data": *tagger_finished.lock().unwrap()
            })).await.ok();
        },
        Action::StopTagging => {
            onetagger_autotag::STOP_TAGGING.store(true, Ordering::SeqCst);
        },
        Action::Waveform { path } => {
            let path = paths::confine(&path)?;
            let source = AudioSources::from_path(&path)?;
            let (waveform_rx, cancel_tx) = source.generate_waveform(180)?;
            // Streamed
            for wave in waveform_rx {
                send_socket(websocket, json!({
                    "action": "waveformWave",
                    "wave": wave
                })).await.ok();
                // Check reply
                if websocket.recv().await.is_none() {
                    cancel_tx.send(true).ok();
                }
            }
            // Done
            send_socket(websocket, json!({
                "action": "waveformDone",
            })).await.ok();
        },
        // Load player file
        Action::PlayerLoad { path } => {
            let path = paths::confine(&path)?;
            let source = AudioSources::from_path(&path)?;
            // Meta. Best-effort: the title and artist here only label the player
            // bar, so a tag that fails to parse should cost the label, not the
            // ability to play the file.
            let tag = Tag::load_file(&path, false).ok();
            let title = tag.as_ref()
                .and_then(|t| t.tag().get_field(Field::Title))
                .and_then(|i| i.first().map(String::from));
            let artists = tag.as_ref()
                .and_then(|t| t.tag().get_field(Field::Artist))
                .unwrap_or(vec![]);
            // Send to UI
            send_socket(websocket, json!({
                "action": "playerLoad",
                "title": title,
                "artists": artists,
                "duration": source.duration() as u64
            })).await.ok();
            // Load
            context.player.load_file(source);
        },
        //  Controls
        Action::PlayerPlay => context.player.play(),
        Action::PlayerPause => context.player.pause(),
        Action::PlayerSeek { pos } => {
            send_socket(websocket, json!({
                "action": "playerSync",
                "playing": context.player.seek(pos)
            })).await.ok();
        },
        Action::PlayerVolume { volume } => context.player.volume(volume),
        Action::PlayerStop => context.player.stop(),
        // Load quicktag files or playlist
        Action::QuickTagLoad { path, playlist, recursive, separators, limit } => {
            let mut data = QuickTagData::default();
            // Playlist
            if let Some(playlist) = playlist {
                data = QuickTag::load_files(paths::confine_all(&playlist.get_files()?)?, &separators)?;
            }
            // Path
            if let Some(path) = path {
                let path = paths::confine(&path)?.to_string_lossy().to_string();
                if PLAYLIST_EXTENSIONS.iter().any(|e| path.to_lowercase().ends_with(e)) {
                    // The playlist itself is inside the library; what it names
                    // need not be.
                    let tracks = paths::confine_all(&get_files_from_playlist_file(&path)?)?;
                    data = QuickTag::load_files(tracks, &separators)?;
                } else {
                    data = QuickTag::load_files_path(
                        &path, 
                        recursive.unwrap_or(false), 
                        &separators, 
                        0, 
                        limit.map(|l| l.then_some(QUICKTAG_LOAD_LIMIT)).flatten().unwrap_or(usize::MAX)
                    )?;
                }
            }
            send_socket(websocket, json!({
                "action": "quickTagLoad",
                "data": data
            })).await.ok();
        },
        // Save quicktag changes
        Action::QuickTagSave { changes } => {
            // A write, so this matters more than a read: confirm the target is
            // in the library before commit() opens it.
            paths::confine(&changes.path)?;
            let tag = changes.commit()?;
            send_socket(websocket, json!({
                "action": "quickTagSaved",
                "path": &changes.path,
                "file": QuickTagFile::from_tag(&changes.path, &tag)?
            })).await.ok();
        },
        // List dir
        Action::QuickTagFolder { path, subdir } => {
            let (new_path, files) = FileBrowser::list_dir_or_default(path.clone().map(|p| PathBuf::from(p)), subdir, true, false, false)?;
            send_socket(websocket, json!({
                "action": "quickTagFolder",
                "files": files,
                "path": new_path,
            })).await.ok();
        }
        // Library-wide search. Only matched paths are tag-read.
        Action::QuickTagSearch { query, path, separators, limit } => {
            // A search root is a directory the walk descends from, so it needs
            // the same check a listing does.
            let root = search_root(path)?;
            let entries = FileBrowser::search(&root, &query, limit.unwrap_or(SEARCH_LIMIT))?;
            let truncated = entries.len() >= limit.unwrap_or(SEARCH_LIMIT);
            let data = QuickTag::load_files(entries.into_iter().map(|e| e.path).collect(), &separators)?;
            send_socket(websocket, json!({
                "action": "quickTagSearch",
                "data": data,
                "query": query,
                "truncated": truncated
            })).await.ok();
        },
        Action::SpotifyAuthorize { client_id, client_secret } => {
            // Authorize cached
            if let Some(spotify) = Spotify::try_cached_token(&client_id, &client_secret) {
                context.spotify = Some(spotify);
            // Authorize new
            } else {
                let (auth_url, client) = Spotify::generate_auth_url(&client_id, &client_secret)?;
                webbrowser::open(&auth_url)?;
                let spotify = tokio::task::spawn_blocking(move || {
                    Spotify::auth_server(client)
                }).await??;
                context.spotify = Some(spotify);
            }
            send_socket(websocket, json!({
                "action": "spotifyAuthorized",
                "value": true
            })).await.ok();
            debug!("Spotify Authorized!");
        },
        // Check if authorized
        Action::SpotifyAuthorized => {
            send_socket(websocket, json!({
                "action": "spotifyAuthorized",
                "value": context.spotify.is_some()
            })).await.ok();
        },
        // A directory's mtime moves when an entry is added, removed or renamed,
        // and does not move when a file's contents are rewritten -- which is
        // exactly what a file browser needs to know and nothing more. One stat
        // instead of a directory walk, so a client can ask often.
        //
        // `entries` disambiguates two changes landing inside one filesystem
        // timestamp tick. Dotfiles are excluded to match what the browser
        // shows; on a network mount there is usually at least one.
        Action::FolderSignature { path } => {
            let path = paths::confine(&path)?;
            let meta = std::fs::metadata(&path)?;
            if !meta.is_dir() {
                return Err(anyhow!("not a directory: {path:?}"));
            }
            let mtime = meta.modified().ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
            let entries = std::fs::read_dir(&path)?
                .filter_map(|e| e.ok())
                .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                .count();
            send_socket(websocket, json!({
                "action": "folderSignature",
                "path": path.to_string_lossy(),
                "mtime": mtime,
                "entries": entries
            })).await.ok();
        },
        Action::TagEditorFolder { path, subdir, recursive } => {
            let recursive = recursive.unwrap_or(false);
            let (new_path, files) = FileBrowser::list_dir_or_default(path.clone().map(|p| PathBuf::from(p)), subdir, true, true, recursive)?;
            send_socket(websocket, json!({
                "action": "tagEditorFolder",
                "files": files,
                "path": new_path,
                // Stateless
                "recursive": recursive
            })).await.ok();
        },
        // Library-wide search; no tags read, so this stays close to walk cost.
        Action::TagEditorSearch { query, path, limit } => {
            let root = search_root(path)?;
            let files = FileBrowser::search(&root, &query, limit.unwrap_or(SEARCH_LIMIT))?;
            let truncated = files.len() >= limit.unwrap_or(SEARCH_LIMIT);
            send_socket(websocket, json!({
                "action": "tagEditorSearch",
                "files": files,
                "query": query,
                "truncated": truncated
            })).await.ok();
        },
        // Load tags of file
        Action::TagEditorLoad { path } => {
            let data = TagEditor::load_file(&paths::confine(&path)?)?;
            send_socket(websocket, json!({
                "action": "tagEditorLoad",
                "data": data
            })).await.ok();
        },
        // Save changes
        Action::TagEditorSave { changes } => {
            paths::confine(&changes.path)?;
            let _tag = changes.commit()?;
            send_socket(websocket, json!({
                "action": "tagEditorSave"
            })).await.ok();
        },
        // Syntax highlight for renamer
        Action::RenamerSyntaxHighlight { template } => {
            let renamer = Renamer::new(TemplateParser::parse(&template));
            let html = renamer.generate_html(&template);
            send_socket(websocket, json!({
                "action": "renamerSyntaxHighlight",
                "html": html
            })).await.ok();
        },
        // Autocomplete data
        Action::RenamerAutocomplete { template } => {
            let ac = Autocomplete::parse(&template);
            let suggestions = ac.suggest_html();
            send_socket(websocket, json!({
                "action": "renamerAutocomplete",
                "suggestions": suggestions,
                "offset": ac.suggestion_offset()
            })).await.ok();
        },
        // Generate new names but don't rename
        Action::RenamerPreview { config } => {
            let config = confine_renamer(config)?;
            let mut renamer = Renamer::new(TemplateParser::parse(&config.template));
            let files = AudioFileInfo::load_files_iter(&config.path, config.subfolders, None, None);
            let files = renamer.generate(files.take(3), &config).unwrap_or(vec![]);
            send_socket(websocket, json!({
                "action": "renamerPreview",
                "files": files,
            })).await.ok();
        },
        // Start renamer
        Action::RenamerStart { config } => {
            let config = confine_renamer(config)?;
            let mut renamer = Renamer::new(TemplateParser::parse(&config.template));
            let files = AudioFileInfo::load_files_iter(&config.path, config.subfolders, None, None);
            let files = renamer.generate(files, &config)?;
            renamer.rename(&files, &config)?;
            send_socket(websocket, json!({
                "action": "renamerDone",
            })).await.ok();
        },
        // File browser list dir
        Action::FolderBrowser { path, child , base } => {
            // Windows root dir override
            let path = if cfg!(windows) && path.to_string_lossy() == "/" {
                if child.is_empty() {
                    PathBuf::from("/".to_string())
                } else {
                    PathBuf::from(format!("{}\\", child))
                }
            } else {
                paths::confine(PathBuf::from(path).join(child))?
            };

            let e = match base {
                true => FolderBrowser::generate_base(&path)?,
                false => FolderBrowser::list_dir(&path)?
            };

            send_socket(websocket, json!({
                "action": "folderBrowser",
                "entry": e,
                "base": base,
                "path": path
            })).await.ok();
        },

        // Manually tag a file
        Action::ManualTag { config, path } => {
            let path = paths::confine(&path)?;
            // Log config
            info!("Manual tag starting for path: {path:?}");
            TaggerConfigs::AutoTagger(config.clone()).debug_print();

            let rx = tokio::task::spawn_blocking(move || {
                onetagger_autotag::manual_tagger(path, &config)
            }).await.unwrap()?;

            for (platform, r) in rx {
                match r {
                    Ok(matches) => {
                        send_socket(websocket, json!({
                            "action": "manualTag",
                            "platform": platform,
                            "status": "ok",
                            "matches": matches
                        })).await.ok();
                    },
                    Err(e) => {
                        send_socket(websocket, json!({
                            "action": "manualTag",
                            "platform": platform,
                            "status": "error",
                            "error": e.to_string()
                        })).await.ok();
                    },
                }
            }

            // On done
            send_socket(websocket, json!({
                "action": "manualTagDone"
            })).await.ok();
        },
        // Apply the tags from manual tagger
        Action::ManualTagApply { matches, path, config } => {
            let path = paths::confine(&path)?;
            match onetagger_autotag::manual_tagger_apply(matches, path, &config) {
                Ok(_) => {
                    send_socket(websocket, json!({
                        "action": "manualTagApplied",
                        "status": "ok"
                    })).await.ok();
                },
                Err(e) => {
                    error!("Failed applying manual tag: {e}");
                    send_socket(websocket, json!({
                        "action": "manualTagApplied",
                        "status": "error",
                        "error": e.to_string()
                    })).await.ok();
                },
            }
        },

        // Extend every match, then hand them back so the matrix can draw the
        // real values. `path` is confined even though nothing is written: it is
        // still an operator-supplied path reaching the platform sources.
        Action::ManualTagExtend { mut matches, path, config } => {
            let _ = paths::confine(&path)?;
            // Blocking: extension is a network round trip per match, and the
            // socket task must not stall while they run.
            let matches = tokio::task::spawn_blocking(move || {
                onetagger_autotag::manual_tagger_extend(&mut matches, &config);
                matches
            }).await?;
            send_socket(websocket, json!({
                "action": "manualTagExtended",
                "status": "ok",
                "matches": matches
            })).await.ok();
        },

        // What the file already holds, for the manual tagger's header strip.
        //
        // Deliberately not TagEditorLoad, which is the obvious reuse: that
        // carries every picture base64-encoded, which is hundreds of KB on every
        // dialog open for something that only needs to say whether art exists.
        // `has_art` is in the trait for exactly this reason. Fields go through
        // `get_field` rather than the raw tag map so the strip reads the same
        // for ID3, Vorbis and MP4 instead of showing TIT2 to one and TITLE to
        // another.
        Action::ManualTagFileInfo { path } => {
            let path = paths::confine(&path)?;
            // Scoped so the tag is dropped before the await below: `Tag` is not
            // Send, and holding it across one makes the whole future unsendable.
            let info = {
                let tag_wrap = Tag::load_file(&path, false)?;
                let tag = tag_wrap.tag();
                let f = |field: Field| tag.get_field(field);
                json!({
                    "title": f(Field::Title),
                    "artists": f(Field::Artist),
                    "album": f(Field::Album),
                    "albumArtists": f(Field::AlbumArtist),
                    "genres": f(Field::Genre),
                    "styles": f(Field::Style),
                    "label": f(Field::Label),
                    "bpm": f(Field::BPM),
                    "key": f(Field::Key),
                    "isrc": f(Field::ISRC),
                    "catalogNumber": f(Field::CatalogNumber),
                    "version": f(Field::Version),
                    "remixers": f(Field::Remixer),
                    // TagDate is not Serialize, and the strip wants one short
                    // string anyway -- a year-only tag should read "1998", not
                    // "1998-01-01", which would invent a precision the file does
                    // not have.
                    "date": tag.get_date().map(|d| match (d.month, d.day) {
                        (Some(m), Some(day)) => format!("{}-{:02}-{:02}", d.year, m, day),
                        (Some(m), None) => format!("{}-{:02}", d.year, m),
                        _ => format!("{}", d.year),
                    }),
                    "hasArt": tag.has_art(),
                    "format": tag_wrap.format(),
                })
            };
            // Read the real length from the audio, the same way the autotagger
            // does. It cannot come from the player: neither entry point into the
            // manual tagger loads one -- QuickTag only sets the path, and the
            // Tag Editor's button is a bare assignment -- so the player holds
            // this file only if the operator happened to play it first. Nor from
            // the duration *tag*, which is frequently absent and, when present,
            // is only as honest as whatever wrote it.
            let duration = AudioSources::from_path(&path)
                .ok()
                .map(|s| (s.duration() / 1000) as u64);
            let mut info = info;
            info["duration"] = json!(duration);
            send_socket(websocket, json!({
                "action": "manualTagFileInfo",
                "status": "ok",
                "info": info
            })).await.ok();
        },

        // Write a composite the operator built in the matrix view.
        Action::ManualTagApplyComposed { track, path, config } => {
            let path = paths::confine(&path)?;
            match onetagger_autotag::manual_tagger_apply_composed(track, path, &config) {
                Ok(_) => {
                    send_socket(websocket, json!({
                        "action": "manualTagApplied",
                        "status": "ok"
                    })).await.ok();
                },
                Err(e) => {
                    error!("Failed applying composed manual tag: {e}");
                    send_socket(websocket, json!({
                        "action": "manualTagApplied",
                        "status": "error",
                        "error": e.to_string()
                    })).await.ok();
                },
            }
        },


        Action::RepoManifest => {
            send_socket(websocket, json!({
                "action": "repoManifest",
                "manifest": onetagger_autotag::repo::fetch_manifest_async().await?
            })).await.ok();
        },
        Action::InstallPlatform { id, version, is_native } => {
            match onetagger_autotag::repo::install_platform(&id, &version, is_native) {
                Ok(_) => send_socket(websocket, json!({
                    "action": "installPlatform",
                    "status": "ok"
                })).await.ok(),
                Err(e) => {
                    error!("Failed installing platform {id}@{version}: {e}");
                    send_socket(websocket, json!({
                        "action": "installPlatform",
                        "status": "error",
                        "error": e.to_string()
                    })).await.ok()
                },
            };
        },

        
        
    }
   
    Ok(())
}