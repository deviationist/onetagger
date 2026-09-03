import { AutotaggerConfig, Track } from "./autotagger";
import { get1t } from "./onetagger";
import { wsUrl } from "./utils";

/// The config the manual tagger sends with every request.
///
/// Shared rather than built at each call site: the direct-match panel needs the
/// same platform credentials the search does -- Discogs will not answer without
/// its token -- and two copies of this would drift the moment one gained a
/// field.
function buildConfig(): AutotaggerConfig {
    const $1t = get1t();
    let config = JSON.parse(JSON.stringify($1t.config.value));
    config.path = '';
    if ($1t.spotify.value.clientId && $1t.spotify.value.clientSecret) {
        config.spotify = {
            clientId: $1t.spotify.value.clientId,
            clientSecret: $1t.spotify.value.clientSecret,
        }
    }
    return config;
}

class ManualTag {

    ws?: WebSocket;
    busy = false;
    done = false;
    matches: TrackMatch[] = [];
    errors: ManualTagError[] = [];

    
    _resolveSaving?: Function;
    _resolveExtend?: Function;
    _resolveFileInfo?: Function;
    _resolveDirectSearch?: Function;
    _resolveDirectUrl?: Function;
    _resolveDirectFetch?: Function;
    /// Matches the operator named by hand, kept apart from the searched ones.
    ///
    /// Separate so that pressing Start does not throw them away: the point of
    /// the matrix is comparing sources against each other, and a hand-picked
    /// Discogs release next to whatever the search found is exactly the
    /// comparison that direct match exists to make possible. `matches` is
    /// re-seeded from this on reset rather than being the only copy.
    directMatches: TrackMatch[] = [];
    /// What the file already holds, shown for comparison against the results.
    fileInfo?: any;
    /// Whether `extend()` has already run for the current result set.
    extended = false;

    constructor() {}

    /// Reset current state
    ///
    /// `keepDirect` is what makes the two halves compose: starting a search
    /// clears the previous *search*, while a release the operator named by hand
    /// is theirs until they close the dialog.
    reset(keepDirect: boolean = false) {
        if (this.ws) {
            this.ws.close();
            this.ws = undefined;
        }
        if (!keepDirect) this.directMatches = [];
        this.matches = [...this.directMatches];
        this.errors = [];
        this.busy = false;
        // A hand-picked match is a result in its own right: without this the
        // dialog would sit on the config panel with matches it is not showing.
        this.done = this.directMatches.length > 0;
        this.extended = false;
    }

    /// Start tagging a track
    tagTrack(path: string, config: AutotaggerConfig) {
        this.reset(true);
        this.busy = true;

        // Open new WS connection because separate thread
        this.ws = new WebSocket(wsUrl());
        this.ws.addEventListener('message', (ev) => {
            this.onWsMessage(JSON.parse(ev.data));
        });
        this.ws.addEventListener('open', () => {
            this.ws!.send(JSON.stringify({
                action: 'manualTag',
                config: config,
                path
            }))
        });
    }

    /// Apply matches
    async apply(matches: TrackMatch[], path: string, config: AutotaggerConfig) {
        // Send to socket and wait for response
        const $1t = get1t();
        let promise = new Promise((res, rej) => this._resolveSaving = res);
        $1t.send('manualTagApply', { matches, path, config });
        let r = await promise;
        this._resolveSaving = undefined;
        return r;
    }

    /// Load a summary of the tags the file already has.
    ///
    /// Not the Tag Editor's loader, which carries every embedded picture
    /// base64-encoded -- hundreds of KB per open for a strip that only needs to
    /// say whether artwork exists.
    async loadFileInfo(path: string): Promise<any> {
        const $1t = get1t();
        // Cleared here rather than in reset(): reset() clears the *search*, and
        // the file's own tags outlive it -- hitting Start must not blank the
        // strip the results are being compared against. Clearing at load time
        // still stops a new path briefly showing the previous file's tags.
        this.fileInfo = undefined;
        let promise = new Promise<any>((res) => this._resolveFileInfo = res);
        $1t.send('manualTagFileInfo', { path });
        const r = await promise;
        this._resolveFileInfo = undefined;
        this.fileInfo = r?.status === 'ok' ? r.info : undefined;
        return this.fileInfo;
    }

    /// Ask the backend to extend every match, so the matrix draws what a
    /// platform can actually offer rather than only what its search returned.
    ///
    /// `musicbrainz` fetches art from the Cover Art Archive during extension and
    /// `traxsource` takes an album_art parameter, so without this their art
    /// cells read "none" for releases that would in fact gain a cover on apply
    /// -- and an empty art cell is exactly what sends you to another column.
    async extend(path: string, config: AutotaggerConfig): Promise<boolean> {
        const $1t = get1t();
        let promise = new Promise<any>((res) => this._resolveExtend = res);
        $1t.send('manualTagExtend', { matches: this.matches, path, config });
        let r = await promise;
        this._resolveExtend = undefined;
        if (r?.status === 'ok' && Array.isArray(r.matches)) {
            // Replace wholesale rather than merging field by field: the backend
            // returned the same matches in the same order, extended.
            this.matches = r.matches;
            this.extended = true;
            return true;
        }
        return false;
    }

    /// Apply a track composed field by field in the matrix view.
    ///
    /// Separate from `apply` because that one sends a *list* the backend merges
    /// by precedence -- first-selection-wins for scalars, union for arrays --
    /// which cannot express "album from here, art from there".
    async applyComposed(track: Track, path: string, config: AutotaggerConfig) {
        const $1t = get1t();
        let promise = new Promise((res) => this._resolveSaving = res);
        $1t.send('manualTagApplyComposed', { track, path, config });
        let r = await promise;
        this._resolveSaving = undefined;
        return r;
    }

    /// Search one platform for releases.
    ///
    /// One platform, not the fan-out `tagTrack` does: this runs while someone
    /// is typing, and Discogs alone sleeps to keep itself under its rate limit.
    async directSearch(platform: string, query: string): Promise<any> {
        const $1t = get1t();
        let promise = new Promise<any>((res) => this._resolveDirectSearch = res);
        $1t.send('directMatchSearch', { platform, query, config: buildConfig() });
        let r = await promise;
        this._resolveDirectSearch = undefined;
        return r;
    }

    /// Ask which platform owns a URL. Answered from the URL shape alone, so it
    /// costs no network call and needs no credentials.
    async directUrl(url: string): Promise<any> {
        const $1t = get1t();
        let promise = new Promise<any>((res) => this._resolveDirectUrl = res);
        $1t.send('directMatchUrl', { url });
        let r = await promise;
        this._resolveDirectUrl = undefined;
        return r;
    }

    /// Every track on one named release.
    async directFetch(platform: string, id: string): Promise<any> {
        const $1t = get1t();
        let promise = new Promise<any>((res) => this._resolveDirectFetch = res);
        $1t.send('directMatchFetch', { platform, id, config: buildConfig() });
        let r = await promise;
        this._resolveDirectFetch = undefined;
        return r;
    }

    /// Take hand-picked matches into the result set.
    ///
    /// Prepended, not appended: these were chosen rather than scored, so they
    /// lead the list -- and in the list view selection order decides which
    /// source wins a contested field, which makes the first row the default
    /// base of the merge.
    addDirect(matches: TrackMatch[]) {
        this.directMatches.push(...matches);
        this.matches = [...matches, ...this.matches];
        this.done = true;
        // The new rows have not been through `extend()`, so the matrix must not
        // go on believing every column is filled in.
        this.extended = false;
    }

    /// WebSocket message handler
    onWsMessage(json: any) {
        switch (json.action) {
            // New result
            case 'manualTag':
                switch (json.status) {
                    case 'ok':
                        this.addMatches(json.matches, json.platform);
                        break;
                    case 'error':
                        this.errors.push({ platform: json.platform, error: json.error });
                        break;
                }
                break;
            
            // Finished
            case 'manualTagDone':
                this.busy = false;
                this.done = true;
                this.ws?.close();
                this.ws = undefined;
                break;
        }
    }

    /// Add new matches to array
    addMatches(matches: TrackMatch[], platform: string) {
        this.matches.push(...matches.map((m) => {
            m.track.platform = platform;
            return m;
        }));
        this.matches.sort((a, b) => b.accuracy - a.accuracy);
    }

}

/// Matched track
interface TrackMatch {
    accuracy: number;
    track: Track;
    reason: string;
}

interface ManualTagError {
    platform: string;
    error: string;
}

export type { TrackMatch };
export { ManualTag, buildConfig };
