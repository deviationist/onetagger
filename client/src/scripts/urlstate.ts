/// Reflects view state into the URL's query string, so a Tag Editor or Quick
/// Tag view can be linked to, bookmarked, and survive a reload.
///
/// The app uses a hash router, so these land after the route:
///   /#/tageditor?path=/music/on-hold/Vurderes&file=…&sort=created&desc=1
///
/// Direction of travel is deliberately one-way per mount: the URL is *read*
/// once when a view mounts, and *written* on every subsequent change. Watching
/// the route and writing it from the same component is what creates update
/// loops, and since writes use replace() there is no history to navigate back
/// through anyway.
///
/// A value read from the URL takes precedence over the persisted setting --
/// that is the whole point of a link. Absent params fall back to settings, so
/// an ordinary visit behaves exactly as before.

import { useRoute, useRouter } from 'vue-router';

interface UrlState {
    /// A query param as a string, or undefined when absent/empty.
    read: (key: string) => string | undefined,
    /// A query param as a boolean. "1"/"true" are true, "0"/"false" are false.
    readBool: (key: string) => boolean | undefined,
    /// Merge a patch into the query. Debounced, so typing in a filter box does
    /// not queue a replace() per keystroke. Undefined/empty values drop the
    /// param rather than writing an empty one, keeping links tidy.
    write: (patch: Record<string, string | number | boolean | undefined | null>) => void
}

/// `routeMatch` guards writes to one route. Needed because the Quick Tag file
/// browser is mounted by App.vue rather than by the view, so without it that
/// component would write `path` into the Tag Editor's URL too.
function useUrlState(routeMatch?: string): UrlState {
    const route = useRoute();
    const router = useRouter();

    // Snapshot at mount. Later writes must not change what reads return, or a
    // restore that runs after the first write would read back its own output.
    const initial: Record<string, string> = {};
    for (const [k, v] of Object.entries(route.query)) {
        const s = Array.isArray(v) ? v[0] : v;
        if (typeof s === 'string' && s.length > 0) initial[k] = s;
    }

    function read(key: string): string | undefined {
        return initial[key];
    }

    function readBool(key: string): boolean | undefined {
        const v = initial[key];
        if (v === undefined) return undefined;
        if (v === '1' || v === 'true') return true;
        if (v === '0' || v === 'false') return false;
        return undefined;
    }

    let pending: Record<string, string | undefined> = {};
    let timer: any = undefined;

    function flush() {
        timer = undefined;
        if (routeMatch && !router.currentRoute.value.path.includes(routeMatch)) {
            pending = {};
            return;
        }
        const query: Record<string, string> = {};
        for (const [k, v] of Object.entries(router.currentRoute.value.query)) {
            const s = Array.isArray(v) ? v[0] : v;
            if (typeof s === 'string' && s.length > 0) query[k] = s;
        }
        for (const [k, v] of Object.entries(pending)) {
            if (v === undefined || v === '') delete query[k];
            else query[k] = v;
        }
        pending = {};
        // replace(), not push(): these are view state, not navigation, and
        // pushing would bury the previous page under a filter's worth of
        // history entries. The rejection is the benign "duplicated navigation"
        // case when nothing actually changed.
        router.replace({ query }).catch(() => {});
    }

    function write(patch: Record<string, string | number | boolean | undefined | null>) {
        for (const [k, v] of Object.entries(patch)) {
            if (v === undefined || v === null || v === '') pending[k] = undefined;
            else if (typeof v === 'boolean') pending[k] = v ? '1' : undefined;
            else pending[k] = String(v);
        }
        if (timer) clearTimeout(timer);
        timer = setTimeout(flush, 250);
    }

    return { read, readBool, write };
}

export type { UrlState };
export { useUrlState };
