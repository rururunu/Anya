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
    {
      path: "/desktop-pet",
      component: () => import("@/pages/DesktopPet.vue"),
    },
    {
      path: "/computer-use-hud",
      component: () => import("@/pages/ComputerUseHud.vue"),
    },
    {
      path: "/computer-use-banner",
      component: () => import("@/pages/ComputerUseBanner.vue"),
    },
  ],
});

export default router;
