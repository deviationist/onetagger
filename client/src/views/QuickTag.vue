<template>
<div>
    <div class='q-mt-xs q-mb-sm text-grey-5 row'>
        <!-- Search -->
        <div class="col">
            <q-input 
                dense
                v-model='filter'
                :label-slot="true"
                class='q-pl-md qt-search-bar'
                filled
                @update:model-value='onFilterInput()'
            >
                <template v-slot:label>
                    <q-icon name="mdi-magnify" size="xs" class='q-pl-xs'></q-icon>
                </template>
                <template v-slot:append>
                    <q-btn-toggle
                        :model-value='scope'
                        @update:model-value='setScope'
                        dense unelevated no-caps size='sm'
                        toggle-color='primary'
                        text-color='grey-5'
                        :options="[{label: 'Folder', value: 'folder'}, {label: 'Library', value: 'library'}]"
                    >
                        <q-tooltip>Folder filters loaded tracks; Library searches every file under the root</q-tooltip>
                    </q-btn-toggle>
                </template>
            </q-input>
        </div>
    
        <!-- Sort -->
        <div class='col-9 row q-pb-sm text-caption text-weight-medium bg-darker text-grey-6 text-capitalize justify-between clickable' :style='"margin-top: 13px"'>
            <span v-for='(option, i) in sortOptions' :key='"so"+i' @click='sort(option)' class='row q-pl-xs'>
                <div :class='{"text-grey-4 clickable": sortOption == option}'>{{option}}</div>
                <div v-if='sortOption == option'>
                    <q-icon style='margin-bottom: 2px;' name='mdi-arrow-up' v-if='!sortDescending'></q-icon>
                    <q-icon style='margin-bottom: 2px;' name='mdi-arrow-down' v-if='sortDescending'></q-icon>
                </div>
            </span>
        </div>

        <!-- Dense tracks -->
        <div class="col-1 clickable q-mr-md text-right">
            <div class='q-pr-xs'>
                <q-btn 
                    :icon='$1t.settings.value.quickTag.thinTracks ? "mdi-view-list" : "mdi-format-list-bulleted-square"' 
                    round
                    flat
                    size='sm' 
                    color='text-grey-4' 
                    :style='"margin-top: 8px"'
                    @click='$1t.settings.value.quickTag.thinTracks = !$1t.settings.value.quickTag.thinTracks'
                ></q-btn>
            </div>
        </div>
    </div>

    <!-- Stats -->
    <div class='q-mx-lg text-grey-7 q-my-xs text-caption text-center' v-if='$1t.quickTag.value.searchQuery !== undefined'>
        Library results: <span class='monospace text-bold'>{{$1t.quickTag.value.tracks.length}}</span>
        <span class='q-ml-sm'>for</span> <span class='monospace text-bold'>{{$1t.quickTag.value.searchQuery}}</span>
        <span v-if='$1t.quickTag.value.searchTruncated' class='q-ml-md'>
            capped at <span class='monospace text-bold'>{{ SEARCH_LIMIT }}</span> &mdash; narrow the search
        </span>
        <span class='q-ml-md text-weight-medium show-link cursor-pointer' @click='backToFolder()'>&larr; back to folder</span>
    </div>
    <div class='q-mx-lg text-grey-7 q-my-xs text-caption text-center' v-else>
        Loaded files: <span class='monospace text-bold'>{{$1t.quickTag.value.tracks.length}}</span>
        <span class='q-ml-md'>Filtered: </span><span class='monospace text-bold'>{{tracks.length}}</span>
        <span v-if='$1t.quickTag.value.failed.length != 0'><span class='q-ml-md'>Failed to load: </span>
            <span class='monospace text-bold' @click='failedDialog = true'>{{$1t.quickTag.value.failed.length}} 
                <span class='text-weight-medium show-link cursor-pointer'>Show details</span>
            </span>
        </span>
        
        <span class='q-ml-md text-caption cursor-pointer' :style='"margin-left: 13px"' v-if='$1t.quickTag.value.isLimited()' @click='$1t.loadQuickTag(undefined, false)'>
            Loading was capped to <span class='text-caption monospace text-bold'>{{ QUICKTAG_LOAD_LIMIT }}</span> tracks! <span class='q-ml-xs text-weight-medium show-link cursor-pointer'>Show all</span>
        </span>
    </div>


    <!-- Tracks -->
    <div class='tracklist qt-full-height' v-if='$1t.quickTag.value.tracks.length > 0' ref='tracklist' :class='{"qt-height": $1t.quickTag.value.track}' @scroll='onScroll'>
        
        <!-- Tracklist -->
        <div v-for='item in tracks' :key='item.path' v-if='!$1t.settings.value.quickTag.thinTracks'>
            <q-intersection style='height: 116px;' @click.native='(e: MouseEvent) => trackClick(item, e)' once>
                <QuickTagTile :track='item' :no-art-cache="noArtCacheList.includes(item.path)"></QuickTagTile>
                <QuickTagContextMenu 
                    @manual-tag="onManualTag(item.path)"
                    :path="item.path"
                ></QuickTagContextMenu>
            </q-intersection>
        </div>
        <!-- Thin tracks -->
        <div :style='`width: ${tracklistWidth}`'>
            <div v-for='(item, i) in tracks' :key='item.path' v-if='$1t.settings.value.quickTag.thinTracks'>
                <q-intersection style='height: 32px;' @click.native='(e: MouseEvent) => trackClick(item, e)' once>
                    <QuickTagTileThin :track='item' :odd='i % 2 == 1'></QuickTagTileThin>
                    <QuickTagContextMenu 
                        @manual-tag="onManualTag(item.path)"
                        :path="item.path"
                    ></QuickTagContextMenu>
                </q-intersection>
            </div>
        </div>

        <!-- No results -->
        <div v-if='tracks.length == 0'>
            <div class='text-center text-h5 text-grey-6 q-my-lg'>No results!</div>
        </div>
    </div>

    <div v-if='$1t.quickTag.value.tracks.length == 0' class='qtbg-container qt-full-height'>
        <!-- Loading -->
        <div v-if='$1t.lock.value.locked' class='row justify-center'>
            <q-circular-progress indeterminate color='primary' size='64px'></q-circular-progress>
        </div>

        <!-- No path selected -->
        <div @click='selectFolder' v-if='!$1t.lock.value.locked'>
            <div class='text-center text-subtitle2 text-bold text-primary q-my-sm'>NO FOLDER SELECTED</div>
            <div class='text-center text-subtitle2 text-grey-6'><span class='keybind-icon q-px-sm text-caption text-bold'>CLICK</span> here to select folder</div>
            
            <div class="q-pa-lg q-mt-lg">
                <div class="row q-py-xs text-caption text-grey-6 text-weight-medium text-right">
                    <div class="col">
                        Play / Pause:
                    </div>

                    <div class="col text-body text-weight-bold text-grey-6 text-left">
                    <q-icon name='mdi-keyboard-space' class='keybind-icon'></q-icon>
                    </div>            
                </div>

                <div class="row text-body q-py-xs text-caption text-grey-6 text-weight-medium text-right">
                    <div class="col">
                        Seek back / forwards:        
                    </div>
                    <div class="col text-body text-weight-bold text-grey-6 text-left">
                        <q-icon name='mdi-chevron-left' class='keybind-icon q-mr-xs'></q-icon> / <q-icon name='mdi-chevron-right' class='keybind-icon'></q-icon>
                    </div>           
                </div>
                <div class="row text-body q-py-xs text-caption text-grey-6 text-weight-medium text-right">
                    <div class="col">
                        Switch Track:
                    </div>
                    <div class="col text-body text-weight-bold text-grey-6 text-left">
                        <q-icon name='mdi-chevron-up' class='keybind-icon q-mr-xs'></q-icon> / <q-icon name='mdi-chevron-down' class='keybind-icon'></q-icon>
                    </div>           
                </div>
                <div class="row text-body q-py-xs text-caption text-grey-6 text-weight-medium text-right">
                    <div class="col">
                        Select multiple:
                    </div>
                    <div class="col text-body text-weight-bold text-grey-6 text-left">
                        <q-icon name='mdi-apple-keyboard-control' class='keybind-icon q-mr-xs'></q-icon> + <span class='keybind-icon q-px-sm'>CLICK</span>
                    </div>           
                </div>
                <div class="row text-body q-py-xs text-caption text-grey-6 text-weight-medium text-right">
                    <div class="col">
                        Save:      
                    </div>
                    <div class="col text-body text-weight-bold text-grey-6 text-left">
                        <q-icon name='mdi-apple-keyboard-control' class='keybind-icon q-mr-xs'></q-icon> + <span class='keybind-icon q-px-sm'>S</span>
                    </div>           
                </div>
                <div class="row text-body q-py-xs text-caption text-grey-6 text-weight-medium text-right">
                    <div class="col">
                        Delete:
                    </div>
                    <div class="col text-body text-weight-bold text-grey-6 text-left">
                        <q-icon name='mdi-apple-keyboard-control' class='keybind-icon q-mr-xs'></q-icon> + <span class='keybind-icon q-px-sm'>DEL</span>
                    </div>           
                </div>
                <div class="row text-body q-py-xs text-caption text-grey-6 text-weight-medium text-right">
                    <div class="col">
                        Confirm:
                    </div>
                    <div class="col text-body text-weight-bold text-grey-6 text-left">
                        <q-icon name='mdi-keyboard-return' class='keybind-icon'></q-icon>
                    </div> 
                </div>
                <div class="row text-body q-py-xs text-caption text-grey-6 text-weight-medium text-right">
                    <div class="col">
                        Context Menu:
                    </div>
                    <div class="col text-body text-weight-bold text-grey-6 text-left">
                        <span class='keybind-icon q-pl-sm'>RIGHT</span><span class='keybind-icon q-px-sm'>CLICK</span>
                    </div> 
                </div>
            </div>
        </div>
    </div>
            
    
    <!-- Save dialog -->
    <q-dialog v-model='saveDialog'>
        <q-card>
            <q-card-section>
                <div class='text-h6 text-grey-3'>Unsaved changes</div>
            </q-card-section>
            <q-card-section>
                <div class='text-grey-3'>Warning, there are unsaved changes on this track, do you want to save them?</div>
            </q-card-section>
            <q-card-actions align='right'>
                <q-btn color='red' flat text @click='saveDialogCallback(false)'>Discard</q-btn>
                <q-btn color='primary' flat text @click='saveDialogCallback(true)' ref='saveButton'>Save</q-btn>
            </q-card-actions>
        </q-card>
    </q-dialog>

    <!-- Note tag dialog -->
    <q-dialog v-model='noteDialog' @show='onNoteDialogShow'>
        <q-card v-if='$1t.quickTag.value.track'>
            <q-card-section>
                <div class='text-h6'>Custom note</div>
            </q-card-section>
            <q-card-section>
                <q-input
                    filled
                    dense
                    label="Note tag"
                    style='width: 256px;'
                    :model-value='$1t.quickTag.value.track.getNote()'
                    @update:model-value='(d: any) => $1t.quickTag.value.track!.setNote(d)'
                    @keyup.enter="noteDialog = false"
                    ref='noteDialogInput'
                ></q-input>
            </q-card-section>
        </q-card>
    </q-dialog>

    <!-- Failed files dialog -->
    <q-dialog v-model='failedDialog'>
        <q-card class='q-pa-md'>
            <q-card-section>
                <div class='text-subtitle2 text-bold text-center text-red'>FAILED TO LOAD</div>
            </q-card-section>
            <q-card-section>
                <div>
                    <div v-for='failed in $1t.quickTag.value.failed' class='q-my-sm'>
                        <div class='text-subtitle3 text-grey-4 monospace'>{{failed.path}}</div>
                        <div class='text-body2 text-red'>{{failed.error}}</div>
                    </div>
                </div>
            </q-card-section>
            <q-card-section horizontal>
                <q-space></q-space>
                <q-btn flat color='primary' @click='failedDialog = false'>Close</q-btn>
            </q-card-section>
        </q-card>
    </q-dialog>

    <!-- Manual Tagger -->
    <ManualTag :path='manualTagPath' @exit='onManualTagDone'></ManualTag>

</div>
</template>

<script lang='ts' setup>
import { scroll, useQuasar } from 'quasar';
import { Ref, computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { get1t } from '../scripts/onetagger.js';
import { useUrlState } from '../scripts/urlstate';
import { CustomTagInfo, QTTrack, QUICKTAG_LOAD_LIMIT, SEARCH_LIMIT } from '../scripts/quicktag.js';

import ManualTag from '../components/ManualTag.vue';
import QuickTagTile from '../components/QuickTagTile.vue';
import QuickTagTileThin from '../components/QuickTagTileThin.vue';
import QuickTagContextMenu from '../components/QuickTagContextMenu.vue';

const { setVerticalScrollPosition } = scroll;

const $1t = get1t();
const $q = useQuasar();
const sortOptions = ['title', 'artist', 'mood', 'energy', 'genre', 'year', 'bpm', 'key', 'custom', 'added', 'modified'];

/// Timestamp a date sort reads, in unix millis. 'added' is the file's birth
/// time -- when the track arrived here -- which unlike mtime cannot be
/// inherited from wherever it was copied from and is not moved by writing
/// tags. It falls back to mtime where the backend could not supply one.
function trackTime(t: any): number {
    if (sortOption.value == 'added') return t.created ?? t.modified ?? 0;
    return t.modified ?? 0;
}
const saveDialog = ref(false);
const noteDialog = ref(false);
const url = useUrlState('quicktag');
const filter = ref<string | undefined>(url.read('filter'));
// 'folder' filters the tracks already loaded (instant, client side); 'library'
// turns the same box into a query answered by the backend. Kept as two explicit
// modes rather than escalating automatically: narrowing a set you have and
// fetching a new one are different operations, and silently switching between
// them makes the box unpredictable.
const scope = ref<'folder' | 'library'>(url.read('scope') == 'library' ? 'library' : 'folder');
// Folder to return to when leaving library mode. settings.path still points at
// it -- library results are not a folder -- so this is only for the label.
const folderBeforeSearch = ref<string | undefined>(undefined);
let searchDebounce: any = undefined;
// Deep link: select this track once the list has loaded.
const pendingUrlTrack = ref<string | undefined>(url.read('track'));
const sortDescending = ref(url.readBool('desc') ?? false);
const sortOption = ref(url.read('sort') ?? 'title');
const failedDialog = ref(false);
const manualTagPath = ref<string | undefined>(undefined);
const noArtCacheList = ref<string[]>([])

let afterSave: undefined | Function = undefined;

// Click on track card
function trackClick(track: QTTrack, event: MouseEvent) {
    // Add track to list
    if (event.ctrlKey || event.metaKey || ($1t.info.value.os == 'macos' && event.altKey)) {
        event.preventDefault();
        event.stopPropagation();
        event.stopImmediatePropagation();
        
        selectionCursor = tracks.value.findIndex(t => t.path == track.path);
        $1t.toggleQTTrack(track);
        return;
    }

    // Expand to add range of tracks to list
    if (event.shiftKey) {
        event.preventDefault();
        event.stopPropagation();
        event.stopImmediatePropagation();

        // No existing selection to expand
        if (selectionCursor === -1) {
            return;
        }

        const currentIndex = tracks.value.findIndex(t => t.path == track.path);
        const startIndex = Math.min(selectionCursor, currentIndex);
        const endIndex = Math.max(selectionCursor, currentIndex);

        for (let i = startIndex; i <= endIndex; i++) {
            $1t.addQTTrack(tracks.value[i]);
        }

        selectionCursor = currentIndex;
        return;
    }

    // Prevent clicking on same track
    if ($1t.quickTag.value.track.isSelected(track)) return;
    selectionCursor = tracks.value.findIndex(t => t.path == track.path);
    url.write({ track: track.path });
    $1t.loadQTTrack(track);
}

// Save dialog callback
async function saveDialogCallback(save: boolean) {
    if (save) {
        await $1t.saveQTTrack();
        $q.notify({
            message: "Tags saved!",
            color: 'primary',
            textColor: 'black',
            timeout: 500,
            position: 'top-right'
        });
    }
    $1t.loadQTTrack(undefined, true);
    saveDialog.value = false;
    // focus on custom tags fix
    setTimeout(() => { $1t.quickTagUnfocus(); }, 50);

    // Do after save action
    if (afterSave) {
        afterSave();
        afterSave = undefined;
    }
}

// Select folder and load tracks
function selectFolder() {
    $1t.browse('qt');
}

// Focus
const noteDialogInput = ref<HTMLElement | undefined>();
function onNoteDialogShow() {
    noteDialogInput.value?.focus();
}

// Sort by option
function sort(option: string) {
    urlWriteSortSoon();
    if (sortOption.value != option) {
        // reset sort direction
        sortDescending.value = false;
        sortOption.value = option;
    } else {
        sortDescending.value = !sortDescending.value;
    }
    // Unselect first
    $1t.quickTag.value.track.removeAll();
    filterTracks();
}

/// Reflect the sort into the URL after the click handler has mutated it.
function urlWriteSortSoon() {
    setTimeout(() => url.write({
        sort: sortOption.value == 'title' ? undefined : sortOption.value,
        desc: sortDescending.value
    }), 0);
}

/// Run the search box. In library scope the text is a query for the backend;
/// in folder scope it filters what is already loaded, exactly as before.
function onFilterInput() {
    url.write({ filter: filter.value });
    if (scope.value == 'library') {
        clearTimeout(searchDebounce);
        // The walk is cheap but each hit costs a tag read, so do not fire per
        // keystroke.
        searchDebounce = setTimeout(() => runLibrarySearch(), 350);
        return;
    }
    filterTracks();
}

/// Issue a library search, or restore the folder when the box is emptied.
function runLibrarySearch() {
    let q = (filter.value ?? '').trim();
    if (!q) {
        $1t.quickTag.value.searchQuery = undefined;
        $1t.loadQuickTag();
        return;
    }
    $1t.searchQuickTag(q);
}

/// Switch between filtering the folder and searching the library.
function setScope(v: 'folder' | 'library') {
    scope.value = v;
    // Default scope leaves no param behind, matching how `bsort` is written.
    url.write({ scope: v == 'library' ? 'library' : undefined });
    if (v == 'library') {
        folderBeforeSearch.value = $1t.settings.value.path;
        runLibrarySearch();
    } else {
        // Leaving library mode: drop the results and reload the folder.
        $1t.quickTag.value.searchQuery = undefined;
        clearTimeout(searchDebounce);
        $1t.loadQuickTag();
    }
}

/// Leave library results and go back to the folder that was open.
function backToFolder() {
    filter.value = undefined;
    url.write({ filter: undefined });
    setScope('folder');
}

/// Filter tracks with search and sorting
function filterTracks() {
    // In library scope the backend has already narrowed the set; filtering it
    // again by the same string would be a no-op at best and would hide hits
    // matched on their folder rather than their tags at worst.
    let inLibrary = $1t.quickTag.value.searchQuery !== undefined;
    let t = (() => {
        let tracks = $1t.quickTag.value.tracks;

        if (filter.value && !inLibrary) {
            let newFilter = filter.value.toLowerCase();
            // title, artist or track or tags
            tracks = $1t.quickTag.value.tracks.filter((t) => 
                t.title.toLowerCase().match(newFilter) || t.path.toLowerCase().match(newFilter) ||
                t.artists.filter((a: any) => a.toLowerCase().match(newFilter)).length > 0 ||
                (t.mood??'').toLowerCase().match(newFilter) ||
                t.getAllCustom().some((i: CustomTagInfo) => i.value.toLowerCase().match(newFilter)) ||
                (t.genres??[]).some((i: any) => i.toLowerCase().match(newFilter)) 
            );
        }
        if (!sortOption.value) return tracks;

        // Sort
        tracks.sort((a, b) => {
            let va, vb;
            switch (sortOption.value) {
                // Timestamps -- numeric, so compared as numbers not strings
                case 'added':
                case 'modified':
                    va = trackTime(a);
                    vb = trackTime(b);
                    break;
                // Arrays
                case 'artist':
                case 'genre':
                    va = a[`${sortOption.value}s`].join(', ').toLowerCase();
                    vb = b[`${sortOption.value}s`].join(', ').toLowerCase();
                    break;
                default:
                    va = (a as any)[sortOption.value]??''.toLowerCase();
                    vb = (b as any)[sortOption.value]??''.toLowerCase();
                    break;
            }

            // Compare
            if (va < vb) {
                return -1;
            }
            if (va > vb) {
                return 1;
            }
            return 0;
        });
        if (sortDescending.value) tracks.reverse();

        return tracks;
    })();

    // Unselect
    if (tracks.value.length != t.length) {
        $1t.quickTag.value.track.removeAll();
    }
    tracks.value = t;

    // Fix width
    fixTracklistWidth();
}

/// Find index of selected track in tracklist
function findIndex(highest: boolean = true) {
    var finalIndex = -1;
    for (let i=0; i < $1t.quickTag.value.track.tracks.length; i++) {
        let index = tracks.value.findIndex(t => t.path == $1t.quickTag.value.track.tracks[i].path);
        // Get at least some index
        if (finalIndex == -1 && index != -1) {
            finalIndex = index;
            continue;
        }
        // Highest index
        if (highest && index > finalIndex) {
            finalIndex = index;
            continue;
        }
        // Lowest index
        if (!highest && index != -1 && index < finalIndex) {
            finalIndex = index;
            continue;
        }
    }
    return finalIndex;
}

// On scroll event
function onScroll(e: Event) {
    // Fix width
    fixTracklistWidth(true);
}

// Open manual tag
async function onManualTag(path: string) {
    // Wait for save
    if ($1t.quickTag.value.track.isChanged()) {
        let promise = new Promise((res, _) => afterSave = res);
        $1t.onQuickTagEvent('onUnsavedChanges');
        await promise;
    }
    $1t.quickTag.value.track.removeAll();

    // Open
    manualTagPath.value = path;
}

// Manual tagging done
function onManualTagDone() {
    noArtCacheList.value.push(manualTagPath.value!);
    manualTagPath.value = undefined;
    $1t.loadQuickTag();
}

/// Export playlist from selected tracks or filtered
function generatePlaylist() {
    let paths = $1t.quickTag.value.track.tracks.map(t => t.path);
    if (paths.length == 0) {
        paths = tracks.value.map(t => t.path);
    }
    $1t.send('generatePlaylist', { paths });
}

// Scroll to track index
const tracklist = ref<HTMLElement | undefined>();
function scrollToIndex(index: number) {
    if ($1t.settings.value.quickTag.thinTracks) {
        setVerticalScrollPosition(tracklist.value!, index * 33 - (tracklist.value!.clientHeight / 68) * 34, 250);
        return;
    }
    setVerticalScrollPosition(tracklist.value!, index * 116 - 154, 250);
}

/// Update tracklist width to fit
const tracklistWidth = ref('100%');
function fixTracklistWidth(force = false) {
    if (force) {
        if (tracklist.value) {
            tracklistWidth.value = `${tracklist.value!.scrollWidth}px`;
        }
        return;
    }
    tracklistWidth.value = '100%';
    setTimeout(() => {
        if (tracklist.value) {
            tracklistWidth.value = `${tracklist.value!.scrollWidth}px`;
        }
    }, 20);
}
let resizeListener = () => fixTracklistWidth();
window.addEventListener('resize', resizeListener);

// Update track list
let tracks: Ref<QTTrack[]> = ref([]);
watch(() => $1t.quickTag.value.tracks, () => filterTracks());

// The view can mount before the socket's `init` arrives, and `init` is what
// supplies libraryRoot. A restored ?scope=library link would then resolve no
// root and silently do nothing, so re-issue once the app reports ready.
watch(() => $1t.info.value.ready, (ready) => {
    if (!ready) return;
    if (scope.value != 'library') return;
    if (!(filter.value ?? '').trim()) return;
    if ($1t.quickTag.value.searchQuery !== undefined) return;
    runLibrarySearch();
});

/// Index of track for selection cursor
let selectionCursor = -1;
let selectionDirection = 0;


const saveButton = ref<any>();
/// Folder signature, so a change can be told from a first look. Cleared when
/// the loaded folder changes, or the new folder's first reply reads as a change.
const lastSignature = ref<string | undefined>(undefined);
let signatureTimer: any = undefined;

function pollFolderSignature() {
    if (document.visibilityState !== 'visible') return;
    const p = $1t.settings.value.path;
    if (!p) return;
    $1t.send('folderSignature', { path: p });
}

onMounted(() => {
    signatureTimer = setInterval(pollFolderSignature, 8000);
    $1t.onQuickTagEvent = (action, data) => {
        switch (action) {
            // Save dialog
            case 'onUnsavedChanges':
                // Autosave enabled
                if ($1t.settings.value.quickTag.autosave) {
                    saveDialogCallback(true);
                    return;
                }

                saveDialog.value = true;
                setTimeout(() => {
                    saveButton.value?.$el.focus()
                }, 100);
                break;

            // Note tag updated
            case 'onNoteTag':
                noteDialog.value = true;
                break;

            // Change track position relatively
            case 'changeTrack':
                var offset = data.offset as number;
                // Get largest index from selected tracks
                var i = findIndex(offset > 0);
                // Load next track
                if (i != -1 && (i + offset) != tracks.value.length && (i + offset) >= 0) {
                    $1t.loadQTTrack(tracks.value[i + offset], data.force??false);
                }
                break;

            // Add track to selection
            case 'addTrack':
                var offset = data.offset as number;
                
                // Update cursor
                if (offset == 0 || $1t.quickTag.value.track.tracks.length == 0) {
                    break;
                }
                if ($1t.quickTag.value.track.tracks.length == 1) {
                    selectionCursor = findIndex();
                }
                var i = selectionCursor;

                // Save directions and offsets to make the shift select working
                var normOffset = Math.min(Math.max(offset, -1), 1);
                if ($1t.quickTag.value.track.tracks.length > 1 && selectionDirection != 0 && selectionDirection != normOffset) {
                    offset = 0;
                }

                // Load next track
                if (i != -1 && (i + offset) != tracks.value.length && (i + offset) >= 0) {
                    // Save correct direction and offset
                    selectionCursor = i + offset;
                    selectionDirection = normOffset;

                    $1t.toggleQTTrack(tracks.value[i + offset]);
                }
                break;

            case 'focusSearch':
                break

            case 'quickTagSearch':
                // Results arrived; re-run sorting so the list obeys the current
                // sort option, exactly as a folder load does.
                filterTracks();
                break;

            case 'quickTagLoad':
                // Deep link: ?track= selects that file once the list is loaded.
                // Must run before the trackIndex restore below, which early-
                // returns when no index was saved -- the common case for a link
                // opened in a fresh session.
                if (pendingUrlTrack.value && $1t.quickTag.value.tracks.length > 0) {
                    const wanted = pendingUrlTrack.value;
                    pendingUrlTrack.value = undefined;
                    const found = $1t.quickTag.value.tracks.find((t: any) => t.path == wanted);
                    if (found) {
                        setTimeout(() => $1t.loadQTTrack(found), 50);
                        break;
                    }
                }
                if ($1t.settings.value.quickTag.trackIndex == -1 || $1t.quickTag.value.tracks.length == 0 || $1t.lock.value.locked) return;
                // Reload last opened track track
                setTimeout(() => {
                    $1t.loadQTTrack($1t.quickTag.value.tracks[$1t.settings.value.quickTag.trackIndex]);
                    $1t.settings.value.quickTag.trackIndex = -1;
                }, 50);

                break;

            case 'onDeleteTrack':
                // Confirm dialog
                $q.dialog({
                    title: 'Delete File',
                    message: 'Permanently delete the selected file(s) from disk? This cannot be undone from OneTagger.',
                    persistent: false,
                    ok: {
                        color: 'red'                        
                    },
                    cancel: {
                        color: ''
                    }
                }).onOk(() => {
                    $1t.player.value.stop();
                    $1t.send('deleteFiles', { paths: $1t.quickTag.value.track.tracks.map(t => t.path) });
                });
                break;

            // Deletes are permanent now, so the list must follow what actually
            // happened rather than a 50ms guess that it did: the acknowledgement
            // carries the paths that really went, and a failure arrives as an
            // error instead, leaving the list alone rather than hiding a track
            // that is still there.
            // The folder changed on disk. Reload through the app's own path so
            // the request carries the separator settings; reconstructing that
            // from outside is what kept this out of the injected script.
            case 'folderSignature':
                if (!data || data.path !== $1t.settings.value.path) break;
                var sig = `${data.mtime}:${data.entries}`;
                if (lastSignature.value === undefined) {
                    lastSignature.value = sig;             // baseline, not a change
                    break;
                }
                if (lastSignature.value === sig) break;
                lastSignature.value = sig;
                // A search result set is not a folder listing; reloading would
                // replace it with one.
                if ($1t.quickTag.value.searchQuery === undefined) $1t.loadQuickTag();
                break;

            case 'deleteFiles':
                if (!data || !data.paths || data.paths.length == 0) break;
                $1t.quickTag.value.track.removeAll();
                $1t.loadQuickTag();
                break;

            case 'quickTagSaved':
                filterTracks();
                break;

            // Manual tag trigger
            case 'onManualTag':
                onManualTag(data.path);
                return;

            case 'generatePlaylist':
                generatePlaylist();
                break;
                
            default:
                console.log(`Unknown QT Event: ${action} ${data}`);
                break;
        }
    }

    // Restore sort state
    sortOption.value = $1t.settings.value.quickTag.sortOption||'title';
    sortDescending.value = $1t.settings.value.quickTag.sortDescending === true;

    // Load tracks if path available. A restored `scope=library` link searches
    // instead -- and must re-issue once settings have arrived over the socket,
    // not only here: `loadSettings` replaces the whole settings object, which
    // would otherwise land after this and reload the folder over the results.
    // Same hazard the file browser works around for its deep-linked selection.
    if (scope.value == 'library' && (filter.value ?? '').trim()) {
        folderBeforeSearch.value = $1t.settings.value.path;
        runLibrarySearch();
    } else {
        $1t.loadQuickTag();
    }
});

onUnmounted(() => {
    if (signatureTimer) { clearInterval(signatureTimer); signatureTimer = undefined; }
    // Save track index if single
    if ($1t.quickTag.value.track.tracks.length == 1) {
        $1t.settings.value.quickTag.trackIndex = $1t.quickTag.value.tracks.findIndex((t) => $1t.quickTag.value.track.tracks[0].path == t.path);
    } else {
        $1t.settings.value.quickTag.trackIndex = -1;
    }

    // Save sorting
    $1t.settings.value.quickTag.sortOption = sortOption.value;
    $1t.settings.value.quickTag.sortDescending = sortDescending.value;

    // Unregister listener
    window.removeEventListener('resize', resizeListener);
});

/// Scroll to position
watch($1t.quickTag.value.track, () => {
    if ($1t.quickTag.value.track.tracks.length != 1) return;
    let index = tracks.value.findIndex((t) => $1t.quickTag.value.track.tracks[0].path == t.path);
    scrollToIndex(index);
});

</script>

<style lang='scss'>
.tracklist {
    overflow-y: auto;
    overflow-x: auto;
}

.qtbg-container {
    display: flex;
    flex-direction: column;
    justify-content: center;
    cursor: pointer;
    margin-top: -30px;
}

.qt-full-height {
    height: calc(100vh - 195px);
}

.qt-height {
    height: calc(100vh - 279px);
}

.keybind-icon {
    padding: 4px;
    border-radius: 2px;
    background: #262828;
    margin-bottom: 4px;
    margin-left: 4px;
}

.bar-bg {
    background: #00ff00;
}

.show-link {
    color: gray;
    text-decoration-line: underline;
}
.show-link:hover {
    color: #f0f0f0;
    text-decoration-line: underline;
}


.qt-search-bar {
    background-color: transparent !important;
    background: transparent !important;
}

.qt-search-bar * {
    background-color: transparent !important;
    background: transparent !important;
}


</style>
