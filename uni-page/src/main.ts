import '@quasar/extras/material-icons/material-icons.css';
import '@quasar/extras/fontawesome-v6/fontawesome-v6.css';
import 'quasar/src/css/index.sass';
import '@/assets/main.scss';

import { createApp } from 'vue';
import { Notify, Quasar } from 'quasar';
import quasarLang from 'quasar/lang/zh-CN';

import App from '@/App.vue';
import router from '@/router';
import pinia from '@/store';

const app = createApp(App);

app.use(pinia);
app.use(router);
app.use(Quasar, {
  plugins: {
    Notify,
  },
  lang: quasarLang,
});

app.mount('#app');
