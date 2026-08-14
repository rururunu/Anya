import { createRouter, createWebHashHistory } from "vue-router";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: "/",
      redirect: "/overlay",
    },
    {
      path: "/overlay",
      component: () => import("@/layouts/Overlay.vue"),
    },
    {
      path: "/workbench",
      component: () => import("@/layouts/Main.vue"),
    },
    {
      path: "/image-preview",
      component: () => import("@/pages/ImagePreview.vue"),
    },
  ],
});

export default router;
