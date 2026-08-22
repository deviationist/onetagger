<template>
<div class='q-px-md'>

    <!-- Path -->
    <div class='text-weight-bold clickable path-display q-my-md' v-if='!editPath'>
        <div class='row inline'>
            <span style="direction:ltr;">
                <span @click='browse' class='text-primary monospace q-pr-xs'>{{path}}</span>
                <q-icon name='mdi-pencil' class='q-pb-xs' @click='editPath = true'></q-icon>
            </span>
        </div>
    </div>
    <div class='q-my-sm' v-if='editPath'>
        <form @submit.prevent='loadFiles()'>
            <q-input outlined dense v-model='path' @blur='editPath = false'></q-input>
        </form>
    </div>


    <div class='q-mt-sm'>

        <!-- Filter -->        
        <q-input dense filled label='Filter' class='q-mb-sm' @update:model-value='applyFilter' v-model='filter'></q-input>

        <!-- Sort -->
        <div class='row items-center q-mb-sm no-wrap'>
            <q-btn-toggle
                v-model='sortMode'
                @update:model-value='onSortChange'
                dense unelevated no-caps size='sm'
                toggle-color='primary'
                text-color='grey-5'
                :options='BROWSER_SORT_OPTIONS'
            ></q-btn-toggle>
            <q-btn
                dense flat size='sm' class='q-ml-xs' text-color='grey-5'
                :icon="sortDescending ? 'mdi-arrow-down' : 'mdi-arrow-up'"
                @click='toggleSortDirection'
            >
                <q-tooltip>{{ sortDirectionLabel(sortMode, sortDescending) }}</q-tooltip>
            </q-btn>
        </div>

        <!-- Parent -->
        <div class='q-mb-sm clickable te-file' v-if='!atRoot' @click='loadFiles("..")'>
            <q-icon size='xs' class='q-mb-xs text-grey-5' name='mdi-folder-upload'></q-icon>
            <span class='q-ml-sm text-caption text-grey-5'>Parent folder</span>
        </div>

        <!-- Files -->
        <div v-for='file in files' :key='file.filename'>
            <div 
                class='clickable te-file' 
                @click='(file.dir || file.playlist) ? loadFiles(file.filename) : loadFiles(file.path)'
                :class='{"text-primary": isSelected(file.path), "text-grey-5": !isSelected(file.path)}'
            >
                <q-icon size='xs' class='q-mb-xs text-grey-5' v-if='!file.dir && !file.playlist' name='mdi-music'></q-icon>
                <q-icon size='xs' class='q-mb-xs text-grey-5' v-if='file.dir' name='mdi-folder'></q-icon>
                <q-icon size='xs' class='q-mb-xs text-grey-5' v-if='file.playlist' name='mdi-playlist-music'></q-icon>
                <span class='q-ml-sm text-caption'>{{file.filename}}</span>
            </div>
        </div>

    </div>

</div>
</template>

<script lang='ts' setup>
import { computed, onMounted, ref } from 'vue';
import { get1t } from '../scripts/onetagger.js';
import { sortBrowserEntries, BROWSER_SORT_OPTIONS, sortDirectionLabel, migrateBrowserSort, atLibraryRoot } from '../scripts/browsersort';
import type { BrowserSort } from '../scripts/browsersort';
import { useUrlState } from '../scripts/urlstate';

const $1t = get1t();
// Guarded to the quicktag route: this component is mounted by App.vue, not by
// the view, so an unguarded write would put `path` in the Tag Editor's URL too.
const url = useUrlState('quicktag');
// The explorer's location and the selected folder are two different things.
// This browser lists *directories only* (the backend passes `files: false`),
// so a folder with no subfolders can be selected -- loading its tracks --
// without the explorer ever navigating into it: the listing comes back empty
// and the navigation half is skipped. Persisting a single `path` therefore
// loses whichever of the two the link actually needed.
//   path   -> where the explorer is browsing
//   folder -> the folder whose tracks are loaded (settings.path)
const urlPath = url.read('path');
const urlFolder = url.read('folder');
// Links written before the two were split carried only `path`, which meant
// the selection; fall back to it so those keep working.
const urlSelection = urlFolder ?? urlPath;
const path = ref(urlPath ?? urlFolder ?? $1t.settings.value.path);
// The server refuses anything above the library root, so do not offer to go there.
const atRoot = computed(() => atLibraryRoot(path.value, $1t.libraryRoot.value));
const files = ref<any[]>([]);
const originalFiles = ref<any[]>([]);
const filter = ref<string | undefined>(url.read('bfilter'));
const initial = ref(true);
const editPath = ref(false);
const sortMode = ref<BrowserSort>(migrateBrowserSort(url.read('bsort') ?? $1t.settings.value.quickTag.browserSort));
const sortDescending = ref<boolean>(url.readBool('bdesc') ?? ($1t.settings.value.quickTag.browserSortDescending === true));

/// Re-sort what is already loaded; no round trip to the backend needed.
function resort() {
    originalFiles.value = sortBrowserEntries(originalFiles.value, sortMode.value, sortDescending.value) as any[];
    applyFilter();
}

function onSortChange() {
    $1t.settings.value.quickTag.browserSort = sortMode.value;
    $1t.saveSettings(false);
    url.write({ bsort: sortMode.value == 'name' ? undefined : sortMode.value });
    resort();
}

function toggleSortDirection() {
    sortDescending.value = !sortDescending.value;
    $1t.settings.value.quickTag.browserSortDescending = sortDescending.value;
    $1t.saveSettings(false);
    url.write({ bdesc: sortDescending.value });
    resort();
}

function loadFiles(f?: string) {
    $1t.send('quickTagFolder', {path: path.value, subdir: f});
}

function browse() {
    $1t.browse('qt', path.value);
}

function applyFilter() {
    url.write({ bfilter: filter.value });
    if (!filter.value || filter.value.trim().length == 0) {
        files.value = originalFiles.value;
        return;
    }
    files.value = originalFiles.value.filter(f => f.filename.toLowerCase().includes(filter.value?.toLowerCase()));
}

function isSelected(path: string) {
    return path == $1t.settings.value.path;
}

onMounted(() => {
    // A `path` in the URL is an explicit request for that folder, so it wins.
    // Adopt it into settings too: the track list is loaded from settings.path,
    // so leaving it behind would show one folder's subfolders next to another
    // folder's tracks. Without a URL path, fall back to settings as before.
    if (urlSelection) {
        $1t.settings.value.path = urlSelection;
    } else {
        path.value = $1t.settings.value.path;
    }
    // Register events
    $1t.onQuickTagBrowserEvent = (json) => {
        switch (json.action) {
            case 'quickTagFolder':
                // Selecting a folder loads its tracks. This is separate from
                // navigating into it below, which only happens when the folder
                // has subfolders to list.
                if (!initial.value) {
                    $1t.settings.value.path = json.path;
                    url.write({ folder: json.path });
                    $1t.loadQuickTag();
                } else if (urlSelection) {
                    // Re-apply the deep-linked selection now rather than only at
                    // mount: `loadSettings` replaces the whole settings object
                    // when it arrives over the socket, which would otherwise
                    // discard a selection written earlier. Idempotent.
                    $1t.settings.value.path = urlSelection;
                    $1t.loadQuickTag();
                }
                initial.value = false;

                // Nothing to navigate into -- a leaf folder. The selection above
                // still stands; only the explorer stays where it is.
                if (json.files.length == 0) return;
                originalFiles.value = sortBrowserEntries(json.files, sortMode.value, sortDescending.value) as any[];
                files.value = originalFiles.value;
                path.value = json.path;
                url.write({ path: json.path });
                break;
            case 'pathUpdate':
                initial.value = true;
                // Open at the configured path, not its parent -- see the note on mount below.
                $1t.send('quickTagFolder', { path: $1t.settings.value.path });
        }
    }

    initial.value = true;
    // Open *at* settings.path rather than its parent, matching the Tag Editor.
    // Both views share settings.path, so opening the parent here meant the two
    // browsers started a level apart from the same setting -- and with the
    // server launched via `--path` (ONETAGGER_PATH in the Docker image) that
    // put this one above the configured library root, listing whatever happens
    // to sit alongside it. Showing the configured folder's own subfolders is
    // also the more useful listing: they are the folders holding tracks.
    // Reaching a sibling is still one click on "Parent folder".
    loadFiles();
});

</script>