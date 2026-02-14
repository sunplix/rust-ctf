import { createRouter, createWebHistory } from "vue-router";

import { pinia } from "../stores";
import { useAuthStore } from "../stores/auth";
import LoginView from "../views/LoginView.vue";
import ContestsView from "../views/ContestsView.vue";
import ContestDetailView from "../views/ContestDetailView.vue";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      redirect: "/contests"
    },
    {
      path: "/login",
      name: "login",
      component: LoginView
    },
    {
      path: "/contests",
      name: "contests",
      component: ContestsView,
      meta: { requiresAuth: true }
    },
    {
      path: "/contests/:contestId",
      name: "contest-detail",
      component: ContestDetailView,
      props: true,
      meta: { requiresAuth: true }
    }
  ]
});

router.beforeEach(async (to) => {
  const authStore = useAuthStore(pinia);
  authStore.hydrateFromStorage();

  if (authStore.accessToken && !authStore.user) {
    await authStore.syncCurrentUser();
  }

  if (to.name === "login" && authStore.isAuthenticated) {
    return { name: "contests" };
  }

  if (to.meta.requiresAuth && !authStore.isAuthenticated) {
    return {
      name: "login",
      query: {
        redirect: to.fullPath
      }
    };
  }

  return true;
});

export default router;
