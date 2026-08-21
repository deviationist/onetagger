import { TaggingStatusWrap } from './autotagger';

export type ViewMode = 'list' | 'table';

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
