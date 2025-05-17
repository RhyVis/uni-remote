import { createRouter, createWebHashHistory } from 'vue-router';
import MainLayout from '@/layout/MainLayout.vue';

const router = createRouter({
  history: createWebHashHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'layout',
      component: MainLayout,
      children: [
        {
          path: '',
          name: 'home',
          component: () => import('@/pages/play-index/index.vue'),
        },
      ],
    },
  ],
});

export default router;
