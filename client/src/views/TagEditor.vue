<template>
<div class='full-height'>

    <div class='row full-height'>
        <!-- File browser -->
        <div 
            @contextmenu.prevent="" 
            class='q-px-md q-pt-md bg-darker' 
            :class='{"col-4": !$1t.settings.value.tagEditorDouble, "col-3": $1t.settings.value.tagEditorDouble}'
            style='max-height: 100%; overflow-y: scroll;'
        >
            <div class='text-weight-bold text-subtitle2 clickable path-display' @click='browse'>
                <div class='row inline'>
                    <span style="direction:ltr;" class='text-primary monospace'>{{path}}</span>
                </div>
            </div>
            <div class='q-mt-sm'>

                <!-- Filter -->
                <q-input dense filled label='Filter' class='q-mb-sm' @update:model-value='(v: any) => applyFilter(v as string)' v-model='filter'>
                    <template v-slot:append>
                        <q-btn-toggle
                            :model-value='scope'
                            @update:model-value='setScope'
                            dense unelevated no-caps size='sm'
                            toggle-color='primary'
                            text-color='grey-5'
                            :options="[{label: 'Folder', value: 'folder'}, {label: 'Library', value: 'library'}]"
                        >
                            <q-tooltip>Folder filters this directory; Library searches every file under the root</q-tooltip>
                        </q-btn-toggle>
                    </template>
                </q-input>

                <!-- Library results header. Replaces the sort row and the
                     parent link, neither of which means anything for a result
                     set that spans folders. -->
                <div v-if='searchQuery !== undefined' class='q-mb-sm'>
                    <div class='text-caption text-grey-5'>
                        <span class='monospace text-bold'>{{files.length}}</span> results for
                        <span class='monospace text-bold'>{{searchQuery}}</span>
                    </div>
                    <div v-if='searchTruncated' class='text-caption text-grey-7'>capped &mdash; narrow the search</div>
                    <div class='clickable text-caption text-primary q-mt-xs' @click='backToFolder()'>&larr; back to folder</div>
                </div>

                <!-- Sort -->
                <div class='row items-center q-mb-sm no-wrap' v-if='searchQuery === undefined'>
                    <q-btn-toggle
                        v-model='sortMode'
                        @update:model-value='onSortChange'
                        dense unelevated no-caps
                        size='sm'
                        toggle-color='primary'
                        text-color='grey-4'
                        :options='BROWSER_SORT_OPTIONS'
                    ></q-btn-toggle>
                    <q-btn
                        dense flat size='sm' class='q-ml-xs' text-color='grey-4'
                        :icon="sortDescending ? 'mdi-arrow-down' : 'mdi-arrow-up'"
                        @click='toggleSortDirection'
                    >
                        <q-tooltip>{{ sortDirectionLabel(sortMode, sortDescending) }}</q-tooltip>
                    </q-btn>
                </div>

                <!-- Parent -->
                <div class='q-mb-sm clickable te-file' v-if='searchQuery === undefined && !atRoot' @click='loadFiles("..")'>
                    <q-icon size='xs' class='q-mb-xs text-grey-4' name='mdi-folder-upload'></q-icon>
                    <span class='q-ml-sm text-caption text-grey-4'>Parent folder</span>
                </div>

                <draggable 
                    id='fileList' 
                    :move='onFileMove' 
                    group='files' 
                    :list='files' 
                    item-key='filename'
                    @change='onFileDrag'>
                    <template #item='{ element: file }'>
                        <div 
                            class='clickable te-file' 
                            @click='(file.dir || file.playlist) ? loadFiles(file.filename) : loadFile(file.path)'
                            :class='{"text-primary": isSelected(file.path), "text-grey-4": !isSelected(file.path)}'
                        >
                            <q-icon size='xs' class='q-mb-xs text-grey-4' v-if='!file.dir && !file.playlist' name='mdi-music'></q-icon>
                            <q-icon size='xs' class='q-mb-xs text-grey-4' v-if='file.dir' name='mdi-folder'></q-icon>
                            <q-icon size='xs' class='q-mb-xs text-grey-4' v-if='file.playlist' name='mdi-playlist-music'></q-icon>
                            <span class='q-ml-sm text-caption' v-if='searchQuery === undefined'>{{file.filename}}</span>
                            <!-- Result rows carry the folder as a second line:
                                 the filename alone does not say which of several
                                 same-named files this is. -->
                            <span class='q-ml-sm text-caption te-result' v-else :title='file.path'>
                                <span class='te-result-path'>{{resultFolder(file.path)}}</span>
                                <span class='te-result-name'>{{file.filename}}</span>
                            </span>

                            <!-- Same right-click idiom as Quick Tag. Files only:
                                 the backend deletes paths, and offering it on a
                                 folder row would imply a recursive delete this
                                 does not do. -->
                            <q-menu v-if='!file.dir && !file.playlist' touch-position context-menu class='no-menu-shadow'>
                                <q-list>
                                    <q-item dense clickable v-close-popup @click='confirmDelete(file.path)'>
                                        <q-item-section avatar>
                                            <q-icon name='mdi-delete' color='red'></q-icon>
                                        </q-item-section>
                                        <q-item-section class='text-red'>
                                            Delete
                                        </q-item-section>
                                    </q-item>
                                </q-list>
                            </q-menu>
                        </div>
                    </template>

                    
                </draggable>
            </div>
        </div>

        <!-- Custom list -->
        <div 
            @contextmenu.prevent="" 
            class='col-3 bg-darker q-px-md q-pt-sm' 
            v-if='$1t.settings.value.tagEditorDouble'
            style='max-height: 100%; overflow-y: scroll;'
        >
            <div class='bg-darker separator'></div>
            <div class='row justify-between'>
                <div class='text-weight-bold text-subtitle2 text-primary q-pb-sm'>Your list</div>
                <div>
                    <q-btn round dense size='xs' flat style='margin-right: 2px;' @click='clearCustom'>
                        <q-icon name='mdi-close' color='red'></q-icon>
                    </q-btn>
                </div>
            </div>
            
            <draggable 
                group='files' 
                :move='onFileMove' 
                :list='customList' 
                @change='onFileDrag' 
                style='height: calc(100% - 32px)'
                :item-key="(e: any) => `//CUSTOM${e}`"
            >
                <template #item='{ element: f }'>
                    <div class='row'>
                        <div 
                            @click='loadFile(f)' 
                            class='te-file clickable q-my-xs q-mr-sm' 
                            style='width: calc(100% - 32px)' 
                            :class='{"text-primary": isSelected(f), "text-grey-4": !isSelected(f)}'
                        >
                            <span>{{filename(f)}}</span>
                        </div>
                        <div>
                            <q-btn size='xs' class='q-mt-xs' flat round style='float: right;' @click='removeCustom(f)'>
                                <q-icon name='mdi-close' color='red'></q-icon>
                            </q-btn>
                        </div>
                    </div>
                </template>
            </draggable>
        </div>

        <!-- Tags -->
        <div 
            :class='{"col-8": !$1t.settings.value.tagEditorDouble, "col-6": $1t.settings.value.tagEditorDouble}'
            style='max-height: 100%; overflow-y: scroll;'>
            <div v-if='!file' class='justify-center items-center content-center row full-height'>
                
                <div class='col-12 text-subtitle2 text-bold text-primary text-center q-my-sm'>NO FILE SELECTED</div><br>
                <span class='text-center text-subtitle2 text-grey-6'>Tip: <span class='keybind-icon q-px-sm text-caption text-bold'>CLICK</span> the path to open a folder and select an audio file</span>
            </div>

            <div v-if='file' class='q-px-md'>
                <div class='row items-center justify-center q-py-md no-wrap'>
                    <div class='text-subtitle2 text-grey-5 monospace selectable'>{{file.filename}}</div>
                    <q-btn round dense flat class='q-ml-sm' :disable='!canCopy' @click='copyFilename'>
                        <q-icon name='mdi-content-copy' size='xs' class='text-grey-5'></q-icon>
                        <q-tooltip>{{ canCopy ? 'Copy filename' : 'Copying needs a secure context (open this over https)' }}</q-tooltip>
                    </q-btn>
                    <q-btn round dense flat class='q-ml-sm' @click='confirmDelete(file.path)'>
                        <q-icon name='mdi-delete' size='xs' class='text-red'></q-icon>
                        <q-tooltip>Delete this file</q-tooltip>
                    </q-btn>
                </div>
                <!-- The four fields that hold a track back, in one place.
                     These bind to the very same tags the rows below edit --
                     not a copy of them -- so the two views can never disagree
                     and neither can overwrite the other. -->
                <div class='q-mt-md q-pa-sm rounded-borders' style='background: rgba(255,255,255,0.03)'>
                    <div class='row items-center q-mb-xs'>
                        <div class='text-uppercase text-primary text-caption text-weight-medium col'>Common tags</div>
                        <div class='text-caption text-grey-7'>{{commonTagNames}}</div>
                    </div>
                    <div class='row q-col-gutter-sm'>
                        <q-input
                            v-for='f in COMMON_FIELDS'
                            :key='f.key'
                            :model-value='file.tags[commonTagName(f)]'
                            @update:model-value='(v: any) => commonInput(f, v)'
                            :label='f.label'
                            filled
                            dense
                            class='col-6'
                            @keyup.enter='save'
                        ></q-input>
                    </div>
                    <div class='row items-center justify-end q-mt-sm'>
                        <div class='text-caption text-grey-7 q-mr-md' v-if='changes.length'>{{changes.length}} unsaved</div>
                        <q-btn dense flat color='primary' label='Save' :disable='!changes.length' @click='save'></q-btn>
                    </div>
                </div>

                <div class='q-mt-md'>
                    <div v-for='(tag, i) in Object.keys(file.tags)' :key='i' class='row q-my-sm'>
                        <div class='col-3 text-body2 text-uppercase text-primary text-weight-medium q-mt-sm q-pr-xs' style='text-overflow: ellipsis; overflow: hidden;'>
                            
                            <span v-if='ABSTRACTIONS[tag]'><span class='text-uppercase text-primary text-weight-medium'>{{ABSTRACTIONS[tag]}} </span><span class="text-grey-4 monospace text-caption"> {{tag}}</span></span>
                            <span v-if='!ABSTRACTIONS[tag]'>{{tag}}</span>
                        </div>
                        
                        
                        <q-input
                            v-model='file.tags[tag]'
                            filled
                            dense
                            class='col-8'
                            @change='onChange(tag)'
                        ></q-input>

                        <div class='col-1 q-pl-md q-pt-xs'>
                            <q-btn round dense flat @click='removeTag(tag)'>
                                <q-icon name='mdi-delete' class='text-red'></q-icon>
                            </q-btn>
                        </div>
                    </div>
                </div>
                <q-separator class='q-mx-auto' :style='"max-width: 513px; margin-top: 40px;"' inset color="dark"/>
                
                <!-- Add new tag -->
                <div class='row q-mt-lg' style='margin-top: 40px;'>
                    <div class='col-3 q-pt-sm text-weight-medium text-grey-4 text-body2'>Add new text tag</div>
                    <TagField tageditor class='col-8' dense :format='tagFormat!' @update:model-value='newTag = $event'></TagField>
                    <div class='col-1 q-pl-md q-pt-xs'>
                        <q-btn round dense flat @click='addNewTag'>
                            <q-icon name='mdi-plus' class='text-primary'></q-icon>
                        </q-btn>
                    </div>
                </div>
                <q-separator class='q-mx-auto' :style='"max-width: 513px; margin-top: 20px; margin-bottom: 25px;"' inset color="dark"/>
                
                <!-- Album art -->
                <div class='text-uppercase text-primary text-weight-medium'>
                    Album art
                    <q-btn round flat class='q-mb-xs q-ml-sm' @click='addAlbumArtDialog = true'>
                        <q-icon name='mdi-plus' color='primary'></q-icon>
                    </q-btn>
                </div>
                <div class='text-grey-4 albumart-container text-center'>
                    <div v-for='(image, i) in file.images' :key='"art"+i' class='q-mr-md'>
                        <!-- <q-img :src='image.data' class='albumart clickable' @click='albumArt = image.data; showAlbumArt = true'></q-img>
                        <div class='q-pt-sm q-mb-md'>
                            <div v-if='file.format != "mp4"' class='text-caption'>{{image.kind}}</div>
                            <div v-if='file.format != "mp4"' class='text-caption'>{{image.description}}</div>
                            <div class='text-subtitle3 text-grey-6 monospace'>{{image.mime}} {{image.width}}x{{image.height}}</div>
                            <q-btn dense push color='red' class='rounded-borders q-px-md q-mt-sm text-weight-medium' @click='removeArt(i)'>Remove</q-btn>
                        </div> -->
                        <TagEditorAlbumArt 
                            :image='image' 
                            @click='albumArt = image.data; showAlbumArt = true' 
                            @remove='removeArt(i)'
                            @replace='addAlbumArt'
                        ></TagEditorAlbumArt>
                    </div>
                </div>

                <!-- ID3 specific tags -->
                <div v-if='file.id3'>
                    <!-- Comments -->
                    <div class='text-uppercase text-primary text-weight-medium'>
                        Comments <span class="text-grey-4 monospace text-caption q-pl-xs">COMM</span>
                        <q-btn round flat class='q-mb-xs q-ml-sm' @click='addID3Comment'>
                            <q-icon name='mdi-plus' color='primary'></q-icon>
                        </q-btn>
                    </div>
                    <div>
                        <div v-for='(comment, i) in file.id3.comments' :key='"comm"+i' class='row q-py-sm'>
                            <q-input
                                filled
                                dense
                                label='Language'
                                class='col-2'
                                v-model='file.id3.comments[i].lang'
                                maxlength='3'
                                @change='id3CommentsChange'
                            ></q-input>
                            <q-input
                                filled
                                dense
                                label='Description'
                                class='col-4 q-pl-sm'
                                v-model='file.id3.comments[i].description'
                                @change='id3CommentsChange'
                            ></q-input>
                            <q-input
                                filled
                                dense
                                label='Text'
                                class='col-5 q-pl-sm'
                                v-model='file.id3.comments[i].text'
                                @change='id3CommentsChange'
                            ></q-input>
                            <div class='col-1 q-pl-md q-pt-xs'>
                                <q-btn round dense flat @click='removeID3Comment(i)'>
                                    <q-icon name='mdi-delete' class='text-red'></q-icon>
                                </q-btn>
                            </div>
                        </div>
                    </div>

                    <!-- Unsynchronized lyrics -->
                    <div class='text-uppercase text-primary text-weight-medium'>
                        Unsynchronized lyrics <span class="text-grey-4 monospace text-caption q-pl-xs">USLT</span>
                        <q-btn round flat class='q-mb-xs q-ml-sm' @click='addID3USLT'>
                            <q-icon name='mdi-plus' color='primary'></q-icon>
                        </q-btn>
                    </div>
                    <div>
                        <div v-for='(lyric, i) in file.id3.unsync_lyrics' :key='"uslt"+i' class='q-py-sm'>
                            <div class='row'>
                                <q-input
                                    filled
                                    dense
                                    label='Language'
                                    class='col-3'
                                    v-model='file.id3.unsync_lyrics[i].lang'
                                    maxlength='3'
                                    @change='id3USLTChange'
                                ></q-input>
                                <q-input
                                    filled
                                    dense
                                    label='Description'
                                    class='col-8 q-pl-md'
                                    v-model='file.id3.unsync_lyrics[i].description'
                                    @change='id3USLTChange'
                                ></q-input>
                                <div class='col-1 q-pl-md q-pt-xs'>
                                    <q-btn round dense flat @click='removeID3USLT(i)'>
                                        <q-icon name='mdi-delete' class='text-red'></q-icon>
                                    </q-btn>
                                </div>
                            </div>
                            <q-input
                                filled
                                dense
                                label='Text'
                                v-model='file.id3.unsync_lyrics[i].text'
                                type='textarea'
                                class='q-pt-sm q-pb-sm'
                                @change='id3USLTChange'
                            ></q-input>
                        </div>
                    </div>

                    <!-- Popularimeter -->
                    <div>
                        <div class='text-uppercase text-primary text-weight-medium'>
                            Popularimeter <span class="text-grey-4 monospace text-caption q-pl-xs">POPM</span>
                            <q-btn v-if='!file.id3.popularimeter' round flat class='q-mb-xs q-ml-sm' @click='addPOPM'>
                                <q-icon name='mdi-plus' color='primary'></q-icon>
                            </q-btn>
                        </div>
                        <div v-if='file.id3.popularimeter' class='row q-py-sm'>
                            <q-input
                                filled
                                dense
                                label='Email'
                                class='col-4'
                                v-model='file.id3.popularimeter.email'
                                @change='id3POPMChange'
                            ></q-input>
                            <q-input
                                filled
                                dense
                                type='number'
                                label='Play count'
                                class='col-3 q-pl-sm'
                                v-model='file.id3.popularimeter.counter'
                                maxlength='9'
                                @change='id3POPMChange'
                            ></q-input>
                            <div class='col-4 q-pl-md'>
                                <q-slider
                                    :min='0'
                                    :max='255'
                                    label
                                    label-text-color='black'
                                    :label-value='POPMLabel'
                                    v-model='file.id3.popularimeter.rating'
                                    @change='id3POPMChange'
                                ></q-slider>
                            </div>
                            <div class='col-1 q-pl-md q-pt-xs'>
                                <q-btn round dense flat @click='removePOPM'>
                                    <q-icon name='mdi-delete' class='text-red'></q-icon>
                                </q-btn>
                            </div>
                        </div>
                    </div>
                    <q-separator class='q-mx-auto' :style='"max-width: 513px; margin-top: 32px; margin-bottom: 25px;"' inset color="dark"/>
                    
                    <!-- ID3v2.4 -->
                    <div class='q-mt-lg text-center'>
                        <div class='text-subtitle2 text-bold text-primary custom-margin'>
                            OPTIONS
                        </div>
                    </div>
                    <div class='column flex-center'>
                        <q-toggle label='Use ID3v2.4' left-label style='width: 160px;' class='justify-between' v-model='id3v24'></q-toggle>
                    </div>
            </div>

            <!-- Close, Manual tag, Save -->
            <q-page-sticky position='bottom-right' :offset='[36, 18]'>
                <div class='row'>
                    <!-- Leftmost, and outline rather than filled: it is the
                         one button here that does not act on the file. -->
                    <q-btn dense
                        outline
                        @click='closeFile'
                        color="grey-5"
                        class='rounded-borders q-px-md q-mt-xs text-weight-medium q-mr-md'
                        label="Close"
                    ></q-btn>

                    <q-btn dense
                        push
                        @click='manualTagPath = file.path'
                        color="primary"
                        class='rounded-borders q-px-md q-mt-xs text-black text-weight-medium q-mr-md'
                        label="Manual Tag"
                    ></q-btn>

                    <q-btn dense
                        push
                        @click='save'
                        color="primary"
                        class='rounded-borders q-px-md q-mt-xs text-black text-weight-medium'
                        label="Save"
                    ></q-btn>
                </div>
            </q-page-sticky>

            </div>
        </div>
    </div>

    <!-- Album art dialog -->
    <q-dialog v-model='showAlbumArt' @hide='albumArt = null'>
        <q-img :src='albumArt' style='max-width: 50%;'></q-img>
    </q-dialog>

    <!-- Add album art dialog -->
    <q-dialog v-model='addAlbumArtDialog'>
        <AddAlbumArt :types='albumArtTypes' @close='addAlbumArtDialog = false' @save='addAlbumArt'></AddAlbumArt>
    </q-dialog>

    <!-- Manual Tag -->
    <ManualTag :path='manualTagPath' @exit='loadFile(manualTagPath!); manualTagPath = undefined;'></ManualTag>

</div>
</template>

<script lang='ts' setup>
import TagField from '../components/TagField.vue';
import AddAlbumArt from '../components/AddAlbumArt.vue';
import draggable from 'vuedraggable';
import { ABSTRACTIONS } from '../scripts/tags';
import { computed, onDeactivated, onMounted, ref } from 'vue';
import { get1t } from '../scripts/onetagger';
import { sortBrowserEntries, BROWSER_SORT_OPTIONS, sortDirectionLabel, migrateBrowserSort, atLibraryRoot } from '../scripts/browsersort';
import { useUrlState } from '../scripts/urlstate';
import type { BrowserSort } from '../scripts/browsersort';
import { useQuasar } from 'quasar';
import ManualTag from '../components/ManualTag.vue';
import TagEditorAlbumArt from '../components/TagEditorAlbumArt.vue';

const $1t = get1t();
const $q = useQuasar();
// A ?path= / ?filter= / ?sort= / ?file= in the URL wins over the persisted
// setting, so a link opens where it points rather than where you last were.
const url = useUrlState('tageditor');
const path = ref(url.read('path') ?? $1t.settings.value.path);
// The server refuses anything above the library root, so do not offer to go there.
const atRoot = computed(() => atLibraryRoot(path.value, $1t.libraryRoot.value));
const files = ref<any[]>([]);
const originalFiles = ref<any[]>([]);
const file = ref<any>(undefined);
const filter = ref<any>(url.read('filter'));
// See the Quick Tag view for why these are two explicit modes rather than one
// box that escalates on its own.
const scope = ref<'folder' | 'library'>(url.read('scope') == 'library' ? 'library' : 'folder');
const searchQuery = ref<string | undefined>(undefined);
const searchTruncated = ref(false);
let searchDebounce: any = undefined;
const changes = ref<any[]>([]);
const newTag = ref<any>(undefined);
const albumArt = ref<any>(undefined);
const showAlbumArt = ref(false);
const addAlbumArtDialog = ref(false);
const customList = ref($1t.settings.value.tagEditorCustom);
const id3v24 = ref(false);
const manualTagPath = ref<string | undefined>(undefined);
const pendingUrlFile = ref<string | undefined>(url.read('file'));
const sortMode = ref<BrowserSort>(migrateBrowserSort(url.read('sort') ?? $1t.settings.value.tagEditorSort));
const sortDescending = ref<boolean>(url.readBool('desc') ?? ($1t.settings.value.tagEditorSortDescending === true));


/// Last known folder signature, so a change can be told from a first look.
/// Reset whenever the folder changes, or the new folder's first reply would
/// read as a change and cause a redundant reload.
const lastSignature = ref<string | undefined>(undefined);
let signatureTimer: any = undefined;

/// Ask whether the folder changed underneath us.
///
/// Polling rather than a filesystem watcher: inotify only reports changes made
/// through the local kernel, so on a network share it would catch some and
/// silently miss others depending on which machine moved the file. Missing
/// half the events while looking like it works is worse than not watching.
///
/// Skipped when the tab is hidden -- a background tab polling a network mount
/// forever is pure waste.
function pollFolderSignature() {
    if (document.visibilityState !== 'visible') return;
    if (!path.value) return;
    $1t.send('folderSignature', { path: path.value });
}

function loadFiles(f?: string) {
    $1t.send('tagEditorFolder', {path: path.value, subdir: f});
}

function browse() {
    $1t.browse('te', path.value);
}

function loadFile(path: string) {
    url.write({ file: path });
    // Autosave
    if (file.value && $1t.settings.value.tagEditorAutosave) {
        save();
    }
    changes.value = [];

    // Will be joined in backend
    $1t.send('tagEditorLoad', {path});
    if ($1t.settings.value.tagEditorPlayer)
        $1t.player.value.loadTrack(path);
}

/// Delete a file, after confirming. The backend moves it to the OS trash
/// (`trash::delete_all`) rather than unlinking it, so on a platform that has a
/// trash this is recoverable -- but where one cannot be created the delete
/// fails outright, which is why nothing here assumes success.
function confirmDelete(target: string) {
    $q.dialog({
        title: 'Delete File',
        message: `Permanently delete ${filename(target)} from disk? This cannot be undone from OneTagger.`,
        persistent: false,
        ok: {
            color: 'red'
        },
        cancel: {
            color: ''
        }
    }).onOk(() => {
        // Release it before it moves -- the player would go on reading a path
        // that is about to stop existing.
        if ($1t.player.value.path == target)
            $1t.player.value.stop();
        $1t.send('deleteFiles', { paths: [target] });
    });
}

/// The backend confirmed the delete. Only now is it safe to drop the open file:
/// a failed delete arrives as an `error` instead, and the editor keeps what it
/// had rather than clearing on optimism and claiming a still-present file is
/// gone.
/// `quiet` suppresses only the notification, not the cleanup.
///
/// The cleanup here -- clear the open file, prune the custom list, refresh the
/// listing -- is worth reusing for any "these paths are gone" event, but the
/// message that goes with it says the files were *deleted*, which is wrong for
/// a caller that moved them and reports that itself. Without this, such a
/// caller either duplicates the cleanup or patches the notification out from
/// the outside; both were tried, and both are worse than one optional flag.
function onDeleted(paths: string[], quiet = false) {
    if (!paths || paths.length == 0) return;

    // Pending edits cannot be written to a file that has moved, and the
    // autosave in loadFile() would try on the very next click.
    if (file.value && paths.includes(file.value.path)) {
        file.value = undefined;
        changes.value = [];
        url.write({ file: undefined });
    }

    // The custom list holds paths, so a deleted one survives there as a row
    // that errors when clicked.
    let before = customList.value.length;
    customList.value = customList.value.filter((p: string) => !paths.includes(p));
    if (customList.value.length != before) saveSettings();

    // Refresh whichever listing is on screen: a search result set does not come
    // back from a folder load.
    if (searchQuery.value !== undefined) runLibrarySearch();
    else loadFiles();

    if (!quiet) {
        $q.notify({
            message: paths.length == 1 ? 'File deleted' : `${paths.length} files deleted`,
            timeout: 2000,
            position: 'top-right'
        });
    }
}

// If file is currently open
function isSelected(path: string) {
    if (!file.value) return false;
    return file.value.path == path;
}

/// Re-sort what's already loaded, without a round trip to the backend
function resort() {
    originalFiles.value = sortBrowserEntries(originalFiles.value, sortMode.value, sortDescending.value) as any[];
    applyFilter(filter.value);
}

function onSortChange() {
    $1t.settings.value.tagEditorSort = sortMode.value;
    $1t.saveSettings(false);
    url.write({ sort: sortMode.value == 'name' ? undefined : sortMode.value });
    resort();
}

function toggleSortDirection() {
    sortDescending.value = !sortDescending.value;
    $1t.settings.value.tagEditorSortDescending = sortDescending.value;
    $1t.saveSettings(false);
    url.write({ desc: sortDescending.value });
    resort();
}

/// Client-side filtering of the loaded directory. Kept separate from
/// applyFilter so the folder-load handler can use it without re-entering the
/// library branch -- going through applyFilter there loops: folder load ->
/// schedule search -> empty query -> folder load.
function applyFolderFilter() {
    if (!filter.value || filter.value.trim().length == 0) {
        files.value = originalFiles.value;
        return;
    }
    files.value = originalFiles.value.filter(f => f.filename.toLowerCase().includes(filter.value.toLowerCase()));
}

function applyFilter(v: string) {
    filter.value = v;
    url.write({ filter: v });
    if (scope.value == 'library') {
        clearTimeout(searchDebounce);
        searchDebounce = setTimeout(() => runLibrarySearch(), 350);
        return;
    }
    applyFolderFilter();
}

/// Issue a library search, or restore the folder listing when emptied.
function runLibrarySearch() {
    let q = (filter.value ?? '').trim();
    if (!q) {
        searchQuery.value = undefined;
        loadFiles();
        return;
    }
    $1t.searchTagEditor(q, path.value);
}

function setScope(v: 'folder' | 'library') {
    scope.value = v;
    url.write({ scope: v == 'library' ? 'library' : undefined });
    if (v == 'library') {
        runLibrarySearch();
    } else {
        clearTimeout(searchDebounce);
        searchQuery.value = undefined;
        files.value = originalFiles.value;
        loadFiles();
    }
}

function backToFolder() {
    filter.value = undefined;
    url.write({ filter: undefined });
    setScope('folder');
}

/// Folder holding a result, relative to the library root. Shown as the first
/// of the two lines on a result row.
function resultFolder(p: string): string {
    let norm = (p ?? '').replace(/\\/g, '/');
    let dir = norm.slice(0, norm.lastIndexOf('/'));
    let root = ($1t.libraryRoot.value ?? '').replace(/\\/g, '/').replace(/\/$/, '');
    if (root && dir.startsWith(root)) {
        let rel = dir.slice(root.length).replace(/^\//, '');
        return rel.length > 0 ? rel : '.';
    }
    return dir;
}


/*
    Custom list
*/

// Vue draggable file drag process
function onFileDrag(e: any) {
    if (e.added) {
        if (e.added.element.dir || e.added.element.playlist) {
            $1t.send('tagEditorFolder', {path: path.value, subdir: e.added.element.filename, recursive: true});
            // Don't copy
            customList.value.splice(e.added.newIndex, 1);
        } else {
            // Duplicate
            if (!customList.value.find((i) => i == e.added.element.path)) 
                customList.value.splice(e.added.newIndex, 1, e.added.element.path);
            else 
                customList.value.splice(e.added.newIndex, 1);
        }
    }
    // Read again
    if (e.removed) {
        files.value.splice(e.removed.oldIndex, 0, e.removed.element);
    }
    saveSettings();
}

// Allow only one way drag
function onFileMove(e: any) {
    if (e.relatedContext.component.$el.id == 'fileList') return false;
}
function removeCustom(i: string) {
    customList.value.splice(customList.value.indexOf(i), 1);
    saveSettings();
}

// Get filename from path
function filename(path: string) {
    path = path.toString();
    if (path.trim().startsWith('/')) {
        let s = path.split('/');
        return s[s.length - 1];
    }
    let s = path.split('\\');
    return s[s.length - 1];
}
function clearCustom() {
    customList.value = [];
    saveSettings();
}

/*
    Common tags
*/

/// The fields that decide whether a track can be published, with the tag each
/// one lives in per format. Not hardcoded to ID3: this editor is used on FLAC
/// as well, where the same field has a different name.
const COMMON_FIELDS = [
    { key: 'title',       label: 'Title',        id3: 'TIT2', vorbis: 'TITLE',       mp4: '\u00A9nam' },
    { key: 'artist',      label: 'Artist',       id3: 'TPE1', vorbis: 'ARTIST',      mp4: '\u00A9ART' },
    { key: 'albumArtist', label: 'Album Artist', id3: 'TPE2', vorbis: 'ALBUMARTIST', mp4: 'aART' },
    { key: 'album',       label: 'Album',        id3: 'TALB', vorbis: 'ALBUM',       mp4: '\u00A9alb' },
];

function commonTagName(f: any): string {
    return f[tagFormat.value ?? 'id3'];
}

/// Shown beside the heading so it stays obvious which raw tags are being
/// written -- this is a shortcut past the frame names, not a hiding of them.
const commonTagNames = computed(() =>
    COMMON_FIELDS.map(f => commonTagName(f)).join(' \u00B7 '));

/// Record the edit as it is typed, rather than waiting for the field to lose
/// focus.
///
/// `onChange` fires on the native change event, which means a value is only
/// staged when the input blurs. That is fine when the next thing you do is
/// click elsewhere, and wrong in the case this form exists for: fill the
/// fields, add artwork, save -- where the last field can still hold focus, or
/// where an earlier field was edited a second time after its first blur, so
/// the staged change carries the older value while the file object has the
/// newer one.
///
/// Staging on every keystroke removes the blur dependency entirely. It costs
/// an array lookup per character, which is nothing next to what it prevents.
function commonInput(f: any, value: any) {
    const tag = commonTagName(f);
    file.value.tags[tag] = value ?? '';
    onChange(tag);
}

/*
    Text Tags
*/

// Delete tag
function removeTag(tag: string) {
    delete file.value.tags[tag];
    changes.value.push({
        type: 'remove',
        tag: tag
    })
}

// Create new tag
function addNewTag() {
    if (!newTag.value) return;
    if (file.value.tags[newTag.value]) {
        $q.notify({
            message: "Tag already exists!",
            timeout: 2000,
            position: 'top-right'
        });
        return;
    }
    // Remove removal of tag
    let i = changes.value.findIndex((c) => c.type == 'remove' && c.tag == newTag.value);
    if (i > -1) changes.value.splice(i, 1);

    file.value.tags[newTag.value] = '';
    changes.value.push({
        type: 'raw',
        tag: newTag.value,
        value: []
    });
}

function onChange(tag: string) {
    let value = file.value.tags[tag]
    // Split only for tags, MP3 write to single tag as id3 separator
    if (file.value.format != 'mp3') {
        value = value.split(',');
    } else {
        value = [value];
    }
    // Generate change
    let index = changes.value.findIndex((c) => c.tag == tag);
    if (index != -1) {
        changes.value[index].value = value; 
    } else {
        changes.value.push({
            type: 'raw',
            tag: tag,
            value: value
        });
    }
}

/*
    Album Art
*/

// Add new album art
function addAlbumArt(data: any) {
    // Find old image
    file.value.images = file.value.images.filter((i: any) => i.kind != data.kind);
    changes.value = changes.value.filter((c) => c.type != 'addPictureBase64' || c.kind != data.kind);

    // Add
    changes.value.push({
        type: 'addPictureBase64',
        mime: data.mime,
        data: data.data,
        kind: data.kind,
        description: data.description
    });
    data.data = `data:${data.mime};base64,${data.data}`;
    file.value.images.push(data);
}

// Delete album art
function removeArt(i: number) {
    let kind = file.value.images[i].kind;
    file.value.images.splice(i, 1);
    //Remove newly added image
    let index = changes.value.findIndex((c) => c.type == "addPictureBase64" && c.kind == kind);
    if (index != -1) {
        changes.value.splice(index, 1);
        return;
    }
    changes.value.push({
        type: 'removePicture',
        kind
    });
}

/*
    ID3 Comments
*/

// Generate new change for ID3 comments
function id3CommentsChange() {
    let i = changes.value.findIndex((c) => c.type == 'id3Comments');
    if (i > -1) {
        changes.value.splice(i, 1);
    }
    changes.value.push({
        type: 'id3Comments',
        comments: file.value.id3.comments
    });
}

function addID3Comment() {
    file.value.id3.comments.push({
        lang: "eng",
        description: "",
        text: ""
    });
    id3CommentsChange();
}

function removeID3Comment(i: number) {
    file.value.id3.comments.splice(i, 1);
    id3CommentsChange();
}


/*
    ID3 Unsynchronized lyrics
*/
function id3USLTChange() {
    let i = changes.value.findIndex((c) => c.type == 'id3UnsynchronizedLyrics');
    if (i > -1) changes.value.splice(i, 1);
    changes.value.push({
        type: 'id3UnsynchronizedLyrics',
        lyrics: file.value.id3.unsync_lyrics
    });
}
function removeID3USLT(i: number) {
    file.value.id3.unsync_lyrics.splice(i, 1);
    id3USLTChange();
}
function addID3USLT() {
    file.value.id3.unsync_lyrics.push({
        lang: 'eng',
        description: '',
        text: ''
    });
    id3USLTChange();
}

/*
    ID3 Popularimeter
*/

function id3POPMChange() {
    // Remove existing popm changes
    let i = changes.value.findIndex((c) => c.type == 'id3Popularimeter');
    if (i > -1) changes.value.splice(i, 1);
    i = changes.value.findIndex((c) => c.type == "remove" && c.tag == "POPM");
    if (i > -1) changes.value.splice(i, 1);
    // Add new changes
    if (file.value.id3.popularimeter) {
        file.value.id3.popularimeter.counter = parseInt(file.value.id3.popularimeter.counter.toString());
        changes.value.push({
            type: 'id3Popularimeter',
            popm: file.value.id3.popularimeter
        });
    } else {
        changes.value.push({
            type: 'remove',
            tag: 'POPM'
        });
    }
}
function addPOPM() {
    file.value.id3.popularimeter = {
        email: "no@email",
        rating: 0,
        counter: 0
    }
    id3POPMChange();
}
function removePOPM() {
    file.value.id3.popularimeter = null;
    id3POPMChange();
}


/*
    Saving and backend
*/

/// Deselect the open file, leaving the folder listing as it is.
///
/// Lives in the sticky footer beside Manual Tag and Save rather than among the
/// filename's icon buttons: those act *on the file*, this acts on the view.
///
/// The same end state the pipeline reaches when a track is moved out from
/// under the editor -- but that path is involuntary and discards pending edits
/// because the file it would write to has gone. This one is a deliberate
/// click on a file that is still there, so unsaved changes are worth a
/// question rather than a silent loss.
function closeFile() {
    let close = () => {
        file.value = undefined;
        changes.value = [];
        url.write({ file: undefined });
    };
    if (!changes.value.length) return close();
    $q.dialog({
        title: 'Close file',
        message: 'This file has unsaved changes. Close it and discard them?',
        cancel: true,
        persistent: false,
        ok: { color: 'red', label: 'Discard' }
    }).onOk(close);
}

/// The Clipboard API is absent outside a secure context, which over plain HTTP
/// to the server's own port is exactly where this runs. Disable and say why,
/// rather than offering a button that throws on click.
const canCopy = computed(() => !!(window.isSecureContext && navigator.clipboard));

/// Copies the filename -- the text shown beside the button -- not the path.
/// `file.path` is right here and is often the more useful thing to paste, but a
/// button that copies something other than what it sits next to is a surprise.
async function copyFilename() {
    if (!file.value) return;
    try {
        await navigator.clipboard.writeText(file.value.filename);
        $q.notify({
            message: 'Filename copied',
            timeout: 2000,
            position: 'top-right'
        });
    } catch (e) {
        $q.notify({
            message: `Could not copy: ${e}`,
            color: 'negative',
            timeout: 4000,
            position: 'top-right'
        });
    }
}

// Save to file
function save() {
    $1t.send('tagEditorSave', {
        changes: {
            path: file.value.path, 
            changes: changes.value,
            separators: {id3: ', ', vorbis: null, mp4: ', '},
            id3v24: id3v24.value
        }
    });
    changes.value = [];
}

function saveSettings() {
    $1t.settings.value.path = path.value;
    $1t.settings.value.tagEditorCustom = customList.value;
    $1t.saveSettings(false);
}

// Websocket callback
function wsCallback(e: any) {
    switch (e.action) {
        case 'browse':
            path.value = e.path;
            loadFiles();
            break;
        // The folder changed on disk. Re-list it, unless a search result set is
        // on screen -- that is not a folder listing and a reload would replace
        // it with one.
        case 'folderSignature':
            if (e.path !== path.value) break;          // a stale reply for a folder we left
            var sig = `${e.mtime}:${e.entries}`;
            if (lastSignature.value === undefined) {
                lastSignature.value = sig;             // first sighting is a baseline
                break;
            }
            if (lastSignature.value === sig) break;
            lastSignature.value = sig;
            if (searchQuery.value === undefined) loadFiles();
            break;
        case 'tagEditorFolder':
            if (e.recursive) {
                // Add dir to custom list
                let files = customList.value.concat(e.files.sort((a: any, b: any) => {
                    return a.filename.toLowerCase().localeCompare(b.filename.toLowerCase());
                }).map((f: any) => f.path));
                // Deduplicate
                customList.value = [... new Set(files)];
            } else {
                path.value = e.path;
                lastSignature.value = undefined;
                url.write({ path: e.path });
                // Dirs first, then per the user's chosen sort mode
                originalFiles.value = sortBrowserEntries(e.files, sortMode.value, sortDescending.value) as any[];
                // A directory listing is not a result set.
                searchQuery.value = undefined;
                searchTruncated.value = false;
                applyFolderFilter();
                // Deep link: open the requested file once the folder holding it
                // has loaded. Guarded so navigating away later doesn't reopen it.
                if (pendingUrlFile.value) {
                    const target = pendingUrlFile.value;
                    pendingUrlFile.value = undefined;
                    loadFile(target);
                }
            }
            saveSettings();
            break;
        case 'tagEditorLoad':
            file.value = e.data;
            break;
        case 'deleteFiles':
            onDeleted(e.paths, e.quiet === true);
            break;
        case 'tagEditorSave':
            $q.notify({
                message: 'Tags written!',
                timeout: 4000,
                position: 'top-right'
            });
            break;
        // Internal callback
        case '_tagEditorSave':
            save();
            break;
        default: 
            console.log(e);
            break;
    }
}

const tagFormat = computed(() => {
    if (!file.value) return null;
    if (file.value.format == 'flac' || file.value.format == 'ogg') return 'vorbis';
    if (file.value.format == 'mp4') return 'mp4';
    return 'id3';
});

// Filter used types
const albumArtTypes = computed(() => {
    let types = ["CoverFront", "CoverBack", "Other", "Artist", "Icon", "OtherIcon", 
        "Leaflet", "Media", "LeadArtist", "Conductor", "Band", "Composer", "Lyricist", 
        "RecordingLocation", "DuringRecording", "DuringPerformance", "ScreenCapture", 
        "BrightFish", "Illustration", "BandLogo", "PublisherLogo"];
    if (!file.value) return types;
    return types.filter((t) => file.value.images.find((i: any) => i.kind == t) ? false : true);
});

const POPMLabel = computed(() => {
    let v = file.value.id3.popularimeter.rating;
    let stars = Math.ceil(v / 51);
    if (stars == 0) stars = 1;
    return `${v} (${stars}⭐)`;
});

// Register callback
// Library search results replace the explorer listing.
$1t.onTagEditorSearchEvent = (json: any) => {
    searchQuery.value = json.query;
    searchTruncated.value = json.truncated;
    files.value = json.files;
};

onMounted(() => {
    signatureTimer = setInterval(pollFolderSignature, 8000);
    $1t.onTagEditorEvent = wsCallback;
    // A restored ?scope=library link searches rather than listing a folder.
    if (scope.value == 'library' && (filter.value ?? '').trim()) {
        runLibrarySearch();
    } else {
        loadFiles();
    }

    // Load QT track
    if ($1t.quickTag.value.toTagEditor) {
        loadFile($1t.quickTag.value.toTagEditor);
        $1t.quickTag.value.toTagEditor = undefined;
    } else if ($1t.quickTag.value.track.tracks.length == 1) {
        loadFile($1t.quickTag.value.track.tracks[0].path);
    }
})

// Unregister
onDeactivated(() => {
    if (signatureTimer) { clearInterval(signatureTimer); signatureTimer = undefined; }
    $1t.onTagEditorEvent = () => {};
})

</script>

<style>
.te-result {
    display: inline-block;
    max-width: calc(100% - 24px);
    vertical-align: top;
}

/* Path first, dimmer: it is context, the filename is the identity. Ellipsised
   from the left so the deepest folder survives truncation. */
.te-result-path {
    display: block;
    font-size: 0.7rem;
    line-height: 1.1;
    opacity: 0.55;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
}

.te-result-name {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.te-file {
    padding: 2px;
    padding-left: 4px;
    border-radius: 8px;
    text-overflow: ellipsis;
    white-space: nowrap;
    overflow: hidden;
}
.te-file:hover {
    background-color: #111312;
}
.path-display {
    text-overflow: ellipsis;
    white-space: nowrap;
    overflow: hidden;
    direction: rtl;
    text-align: left;
}
.albumart {
    min-width: 128px;
    width: 128px;
    max-width: 128px;
    border-radius: 8px;
}
.albumart-container {
    display: flex;
    width: 180px;
}
.separator {
    width: 2px; 
    margin-left: -17px; 
    position: absolute;
    height: 100%;
}
</style>