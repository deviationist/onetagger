import { createRouter, createWebHashHistory } from 'vue-router';

import Index from '../views/Index.vue';
import TagEditor from '../views/TagEditor.vue';
import Renamer from '../views/Renamer.vue';

// Required for hot reload, idk why it broke
const AutotaggerStatus = () => import('../views/AutotaggerStatus.vue');
const Autotagger = () => import('../views/Autotagger.vue');
const QuickTag = () => import('../views/QuickTag.vue');
const AudioFeatures = () => import('../views/AudioFeatures.vue');

const history = createWebHashHistory();

const routes = [
    {
        path: '/',
        component: Index
    },
    {
        path: '/autotagger',
        component: Autotagger,
        meta: { title: 'Auto tag' }
    },
    {
        path: '/autotagger/status',
        component: AutotaggerStatus,
        meta: { title: 'Auto tag status' }
    },
    {
        path: '/quicktag',
        component: QuickTag,
        meta: { title: 'Quick Tag' }
    },
    {
        path: '/audiofeatures',
        component: AudioFeatures,
        meta: { title: 'Audio features' }
    },
    {
        path: '/audiofeatures/status',
        component: AutotaggerStatus,
        meta: { title: 'Audio features status' }
    },
    {
        path: '/tageditor',
        component: TagEditor,
        meta: { title: 'Edit Tags' }
    },
    {
        path: '/renamer',
        component: Renamer,
        meta: { title: 'Auto Rename' }
    }
];

const router = createRouter({
    history,
    routes
});

/// The name shown in the tab, the window switcher and the taskbar.
///
/// Written closed rather than spaced. The project uses both -- upstream is
/// itself split, with the spaced form surviving in a handful of user-visible
/// strings -- and this fork settles on `OneTagger`, the form that owns the
/// repository, the crates, the binary and the image tag.
const APP_NAME = 'OneTagger';

/// Name the tab after the view that is open.
///
/// Every window otherwise reads the same constant, so Quick Tag and Edit Tags
/// open side by side -- the normal way this fork gets used -- cannot be told
/// apart in the tab strip or the window switcher. The view comes first because
/// tab strips truncate from the right, so the distinguishing half is the half
/// that survives.
router.afterEach((to) => {
    const view = to.meta?.title as string | undefined;
    document.title = view ? `${view} - ${APP_NAME}` : APP_NAME;
});

export default router;
