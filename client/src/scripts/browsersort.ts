/// Shared sorting for the file/folder browsers in the Tag Editor and Quick Tag.
///
/// Both browsers are fed `FolderEntry` values from the same backend
/// (`FileBrowser::list_dir_or_default`), so they sort identically and the
/// comparator lives here rather than being duplicated per view.

/// A browser entry as the backend sends it. `modified` and `created` are unix
/// millis; `created` is `null` wherever the platform, filesystem or transport
/// cannot supply a birth time.
interface BrowserEntry {
    filename: string,
    path: string,
    dir: boolean,
    playlist?: boolean,
    modified?: number | null,
    created?: number | null
}

/// 'name' sorts alphabetically. 'created' is when the entry appeared here and
/// is the one to reach for when hunting recent arrivals: it cannot be
/// inherited from wherever a file was copied from, and rewriting tags does not
/// move it. 'modified' is the content date, which a copy may carry over from
/// another machine and which tag writes do move -- on a directory it tracks
/// when entries were last added or removed, which makes it a good "recently
/// worked in" signal.
type BrowserSort = 'name' | 'created' | 'modified';

function nameCompare(a: BrowserEntry, b: BrowserEntry): number {
    return a.filename.toLowerCase().localeCompare(b.filename.toLowerCase());
}

/// The timestamp `mode` sorts on, falling back to mtime when the backend could
/// not supply a birth time for this entry, so the sort degrades per-entry
/// instead of collapsing wholesale.
function timeOf(f: BrowserEntry, mode: BrowserSort): number | null {
    if (mode == 'created') return f.created ?? f.modified ?? null;
    return f.modified ?? null;
}

/// Sort a browser listing, returning a new array. Directories are always
/// pinned above files so navigation stays predictable in every mode. Equal
/// values tiebreak on filename so the order is stable between renders, and
/// entries with no usable timestamp sort last.
function sortBrowserEntries(list: BrowserEntry[], mode: BrowserSort, descending: boolean): BrowserEntry[] {
    return [...list].sort((a, b) => {
        if (a.dir && !b.dir) return -1;
        if (b.dir && !a.dir) return 1;

        if (mode == 'name') {
            const r = nameCompare(a, b);
            return descending ? -r : r;
        }

        const va = timeOf(a, mode), vb = timeOf(b, mode);
        if (va == null && vb == null) return nameCompare(a, b);
        if (va == null) return 1;
        if (vb == null) return -1;
        if (va == vb) return nameCompare(a, b);
        return descending ? vb - va : va - vb;
    });
}

/// Options for the q-btn-toggle both browsers render.
const BROWSER_SORT_OPTIONS = [
    { label: 'Name', value: 'name' },
    { label: 'Added', value: 'created' },
    { label: 'Modified', value: 'modified' }
];

/// Tooltip for the direction button, phrased for the active mode.
function sortDirectionLabel(mode: BrowserSort, descending: boolean): string {
    if (mode == 'name') return descending ? 'Z to A' : 'A to Z';
    return descending ? 'Newest first' : 'Oldest first';
}

/// Migrate the pre-three-mode persisted value, which called mtime 'date'.
function migrateBrowserSort(stored: string | undefined): BrowserSort {
    if (stored == 'date') return 'modified';
    if (stored == 'created' || stored == 'modified' || stored == 'name') return stored;
    return 'name';
}

export type { BrowserEntry, BrowserSort };
export { sortBrowserEntries, BROWSER_SORT_OPTIONS, sortDirectionLabel, migrateBrowserSort };

/// Whether `path` is the library root, so a browser should not offer to go up.
///
/// The server confines file access to the root it was started on, so the parent
/// link at the top of the library leads somewhere the backend will refuse. Better
/// to not offer the move than to explain the error afterwards. With no root
/// configured -- a local run -- nothing is confined and the link always shows.
export function atLibraryRoot(path: string | undefined, root: string | undefined): boolean {
    if (!root || !path) return false;
    const norm = (p: string) => p.replace(/\\/g, '/').replace(/\/+$/, '');
    return norm(path) === norm(root);
}
