import { AutotaggerConfig, Track } from "./autotagger";
import { get1t } from "./onetagger";
import { wsUrl } from "./utils";

class ManualTag {

    ws?: WebSocket;
    busy = false;
    done = false;
    matches: TrackMatch[] = [];
    errors: ManualTagError[] = [];

    
    _resolveSaving?: Function;
    _resolveExtend?: Function;
    _resolveFileInfo?: Function;
    /// What the file already holds, shown for comparison against the results.
    fileInfo?: any;
    /// Whether `extend()` has already run for the current result set.
    extended = false;

    constructor() {}

    /// Reset current state
    reset() {
        if (this.ws) {
            this.ws.close();
            this.ws = undefined;
        }
        this.matches = [];
        this.errors = [];
        this.busy = false;
        this.done = false;
        this.extended = false;
        this.fileInfo = undefined;
    }

    /// Start tagging a track
    tagTrack(path: string, config: AutotaggerConfig) {
        this.reset();
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
export { ManualTag };
