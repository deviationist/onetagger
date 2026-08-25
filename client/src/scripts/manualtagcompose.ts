/// Composing a track field by field, for the manual tagger's matrix view.
///
/// Kept out of the component deliberately. This is the code that decides which
/// tags get written to a file, and getting it wrong does not look like a broken
/// screen -- it looks like a track that quietly lost its title. It is pure, it
/// is tested, and the component is left with the rendering.
import { Track } from './autotagger';

export interface Row {
    key: string;
    label: string;
    /// The SupportedTag this row writes. Dropping it from the config's tag list
    /// is what stops the row being applied -- see `narrowTags`.
    tag: string;
    /// Fields stored separately that only make sense together: a source with a
    /// release *year* but no full date would otherwise contribute half a date,
    /// and artwork carries its own thumbnail URL.
    siblings?: string[];
    hint?: string;
}

export const SCALARS: Row[] = [
    { key: 'title',          label: 'Title',         tag: 'title' },
    { key: 'version',        label: 'Version',       tag: 'version' },
    { key: 'album',          label: 'Album',         tag: 'album' },
    { key: 'label',          label: 'Label',         tag: 'label' },
    { key: 'catalog_number', label: 'Catalogue №',   tag: 'catalogNumber' },
    { key: 'key',            label: 'Key',           tag: 'key' },
    { key: 'bpm',            label: 'BPM',           tag: 'bpm',         hint: 'e.g. 128' },
    { key: 'release_date',   label: 'Release date',  tag: 'releaseDate', siblings: ['release_year'], hint: 'YYYY-MM-DD' },
    { key: 'publish_date',   label: 'Publish date',  tag: 'publishDate', siblings: ['publish_year'], hint: 'YYYY-MM-DD' },
    { key: 'isrc',           label: 'ISRC',          tag: 'isrc' },
    { key: 'mood',           label: 'Mood',          tag: 'mood' },
    { key: 'explicit',       label: 'Explicit',      tag: 'explicit',    hint: 'yes / no' },
    { key: 'art',            label: 'Artwork',       tag: 'albumArt',    siblings: ['thumbnail'], hint: 'image URL' },
    { key: 'url',            label: 'URL',           tag: 'url' },
];

export const ARRAYS: Row[] = [
    { key: 'artists',       label: 'Artists',       tag: 'artist' },
    { key: 'album_artists', label: 'Album artists', tag: 'albumArtist' },
    { key: 'genres',        label: 'Genres',        tag: 'genre' },
    { key: 'styles',        label: 'Styles',        tag: 'style' },
    { key: 'remixers',      label: 'Remixers',      tag: 'remixer' },
];

export const ROWS: Row[] = [...SCALARS, ...ARRAYS];

/// One match index, or the typed Custom column.
export type Choice = number | 'custom' | undefined;

export interface Selection {
    /// Whether the row is applied at all.
    enabled: Record<string, boolean>;
    /// Which source wins each scalar row.
    scalar: Record<string, Choice>;
    /// Which sources contribute to each list row.
    arr: Record<string, (number | 'custom')[]>;
    /// The Custom column's raw text, per row.
    custom: Record<string, string>;
}

/// Does this source offer anything for this field?
///
/// Empty string and empty array count as absent, not as a value -- a source
/// that returns `""` for album has no album, and treating it as one would let
/// it win the row and blank the field.
export function has(track: any, key: string): boolean {
    const v = track[key];
    if (v === undefined || v === null || v === '') return false;
    if (Array.isArray(v)) return v.length > 0;
    return true;
}

/// Parse a typed value into the shape the field expects.
///
/// Returns undefined for anything unusable, which the caller treats as "leave
/// this field unwritten" -- silently writing 0 for an unparseable BPM would be
/// worse than writing nothing.
export function parseCustom(key: string, raw: string): any {
    const v = (raw ?? '').trim();
    if (!v) return undefined;
    if (key === 'bpm') {
        const n = parseInt(v, 10);
        return Number.isFinite(n) ? n : undefined;
    }
    if (key === 'explicit') {
        if (/^(y|yes|true|1)$/i.test(v)) return true;
        if (/^(n|no|false|0)$/i.test(v)) return false;
        return undefined;
    }
    return v;
}

/// Build the track to write, and the list of tags that may be written.
///
/// Two results because they answer different questions. The track carries the
/// values; the tag list decides which of them the backend is allowed to touch,
/// and it is the tag list that makes "only the artwork" safe. Leaving a field
/// empty is NOT equivalent: `write_to_file` writes Title and Artist unguarded --
/// every other field sits behind an is_some()/!is_empty() check and an empty one
/// leaves the file alone -- so an empty title or artist is written, and writing
/// an empty one erases what the file had.
///
/// A row contributes its tag only when it is ticked *and* resolves to a real
/// value. An unticked row, a row with nothing selected, and a row pointed at an
/// empty cell are all the same statement -- do not touch this field -- and the
/// values already on the file survive untouched.
///
/// The base is the highest-accuracy match so the fields the matrix does not
/// arbitrate (platform, duration, ids, arbitrary `other` frames) still carry
/// something coherent. Those ride along in the payload but are not in the
/// returned tag list, so they are not written.
export function compose(matches: { track: Track }[], sel: Selection): { track: Track, tags: string[] } {
    const base: any = JSON.parse(JSON.stringify(matches[0].track));
    const tags: string[] = [];

    for (const f of SCALARS) {
        if (!sel.enabled[f.key]) continue;
        const keys = [f.key, ...(f.siblings ?? [])];
        const choice = sel.scalar[f.key];
        if (choice === undefined) continue;

        if (choice === 'custom') {
            const parsed = parseCustom(f.key, sel.custom[f.key]);
            if (parsed === undefined) continue;
            base[f.key] = parsed;
            // A typed date cannot also supply a year, and a typed artwork URL
            // has no thumbnail -- clear the siblings rather than leaving the
            // primary match's, which would pair a new value with an old one.
            for (const k of keys.slice(1)) base[k] = undefined;
            tags.push(f.tag);
            continue;
        }

        const src: any = matches[choice].track;
        if (!has(src, f.key)) continue;
        for (const k of keys) base[k] = src[k];
        tags.push(f.tag);
    }

    for (const f of ARRAYS) {
        if (!sel.enabled[f.key]) continue;
        const out: string[] = [];
        for (const c of sel.arr[f.key] ?? []) {
            const vals: string[] = c === 'custom'
                ? (sel.custom[f.key] ?? '').split(',').map((s) => s.trim()).filter(Boolean)
                : ((matches[c as number].track as any)[f.key] ?? []);
            for (const v of vals) if (!out.includes(v)) out.push(v);
        }
        if (!out.length) continue;
        base[f.key] = out;
        tags.push(f.tag);
    }

    return { track: base as Track, tags };
}

/// Narrow a config's tag list to the rows being applied.
///
/// An intersection, never a union: a tag the operator disabled in their own
/// settings stays disabled, and tags the matrix has no row for (metaTags,
/// lyrics, ids) pass through untouched rather than being silently dropped.
export function narrowTags(configTags: string[] | undefined, appliedTags: string[]): string[] {
    const owned = new Set(ROWS.map((r) => r.tag));
    return (configTags ?? []).filter((t) => !owned.has(t) || appliedTags.includes(t));
}
