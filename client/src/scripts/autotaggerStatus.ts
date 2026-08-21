import { TaggingStatusWrap } from './autotagger';

export type ViewMode = 'list' | 'table';

/// Where the Autotagger status view mode is remembered.
///
/// Deliberately localStorage rather than the URL or the server-side settings:
/// this is a per-device display preference, not shareable view state like a
/// filter or a sort (those belong in the query string so a link reproduces what
/// the sender saw) and not a cross-device setting worth round-tripping to the
/// config file. Reads are defensive: storage can be unavailable or throw
/// outright in a private window, and a missing or unrecognised value simply
/// falls back to the default.
const VIEW_MODE_KEY = 'onetagger.autotaggerStatus.viewMode';

export function loadViewMode(fallback: ViewMode = 'list'): ViewMode {
    try {
        const v = localStorage.getItem(VIEW_MODE_KEY);
        return v === 'list' || v === 'table' ? v : fallback;
    } catch {
        return fallback;
    }
}

export function saveViewMode(mode: ViewMode) {
    try {
        localStorage.setItem(VIEW_MODE_KEY, mode);
    } catch {
        // Not worth surfacing: the toggle still works for this session.
    }
}

// Convert platform name to display label
export function platformText(p: string) {
    if (p == 'junodownload') return 'JUNO DOWNLOAD';
    if (p == 'audioFeatures') return 'AUDIO FEATURES';
    return p.toUpperCase();
}

export function statusIcon(s: string) {
    switch (s) {
        case 'error': return 'mdi-alert-circle';
        case 'ok': return 'mdi-check';
        case 'skipped': return 'mdi-debug-step-over';
    }
}

export function statusColor(s: string) {
    switch (s) {
        case 'error': return 'red';
        case 'ok': return 'green';
        case 'skipped': return 'yellow';
    }
}

/// Get actual status from status list
export function getStatus(s: TaggingStatusWrap[]): string {
    if (s.find((s) => s.status.status == 'ok')) {
        return 'ok';
    }
    if (s.find((s) => s.status.status == 'skipped')) {
        return 'skipped';
    }
    return 'error';
}

export function countStatus(all: TaggingStatusWrap[][], status: string): number {
    return all.reduce((a, c) => (getStatus(c) == status) ? a + 1 : a, 0);
}
