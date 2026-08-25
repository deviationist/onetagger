import { computed, ComputedRef, Ref } from 'vue';
import { get1t } from './onetagger';

/// Two bands, and only one of them is ours to choose.
///
/// The outer bound is `max_duration_difference`, whose default is 30s: beyond
/// it the matcher would have rejected the candidate outright, so red means
/// "this would not have matched automatically" rather than a taste judgement.
///
/// The inner bound has no equivalent in OneTagger's config, so 15s is a
/// choice: wide enough to absorb the sloppy durations platforms report --
/// rounding, a counted fade, a silent tail -- and tight enough that two mixes
/// of one track rarely both land in it.
export const CLOSE_ENOUGH_SECONDS = 15;
export const MAX_DURATION_DIFFERENCE = 30;

/// The length of the file being tagged, when it is known.
///
/// Read from the file by the backend when the manual tagger opens, which is the
/// only source that is reliably there. The player is not: neither entry point
/// loads one -- QuickTag sets a path, the Tag Editor's button is a bare
/// assignment -- so it holds this file only if the operator happened to play it
/// first, and from the Tag Editor essentially never. Falling back to it anyway
/// costs nothing and covers the window before the info request returns.
///
/// The player fallback stays guarded on the path, because it holds whatever was
/// last loaded and that is not always the file this was opened for. A delta
/// measured against the wrong file would be worse than no delta: it looks like
/// an answer.
export function useFileDuration(path: Ref<string | undefined>): ComputedRef<number | undefined> {
    const $1t = get1t();
    return computed(() => {
        const fromFile = $1t.manualTag.value?.fileInfo?.duration;
        if (typeof fromFile === 'number' && fromFile > 1) return Math.round(fromFile);

        const player = $1t.player.value;
        if (!player?.path || !path.value) return undefined;
        if (player.path !== path.value) return undefined;
        const secs = Math.round(player.duration);
        return secs > 1 ? secs : undefined;
    });
}

/// A platform result's length, as m:ss.
///
/// Absent on some platforms and zero-valued on others, and a length of 0:00 is
/// worse than no length at all -- it reads as a fact rather than a gap -- so
/// both are treated as unknown and the caller shows nothing.
export function trackLength(track: any): string | undefined {
    const secs = track?.duration?.secs;
    if (!secs) return undefined;
    return `${Math.floor(secs / 60)}:${String(secs % 60).padStart(2, '0')}`;
}

/// How far a result is from the file being tagged, in seconds.
///
/// This is the number that actually settles a mix. Two remixes of one track
/// share a title, an artist and often an album, and differ by minutes -- so the
/// absolute length is useful and the *difference* is decisive.
export function lengthDelta(track: any, ours: number | undefined): number | undefined {
    const theirs = track?.duration?.secs;
    if (!theirs || !ours) return undefined;
    return theirs - ours;
}

export function deltaLabel(delta: number): string {
    // An exact match is not "-0:00". Zero has no sign, and printing one makes a
    // perfect match look like a near miss.
    if (delta === 0) return 'exact';
    const sign = delta > 0 ? '+' : '-';
    const a = Math.abs(delta);
    return `${sign}${Math.floor(a / 60)}:${String(a % 60).padStart(2, '0')}`;
}

export function deltaColor(delta: number): string {
    const a = Math.abs(delta);
    if (a <= CLOSE_ENOUGH_SECONDS) return 'text-green-5';
    if (a <= MAX_DURATION_DIFFERENCE) return 'text-orange-5';
    return 'text-red-5';
}
