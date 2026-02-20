import { createRouter, createWebHistory } from "vue-router";

import { pinia } from "../stores";
import { useAuthStore } from "../stores/auth";
import LoginView from "../views/LoginView.vue";
import HomeView from "../views/HomeView.vue";
import ContestsView from "../views/ContestsView.vue";
import ContestDetailView from "../views/ContestDetailView.vue";
import ScoreboardWallView from "../views/ScoreboardWallView.vue";
import AdminView from "../views/AdminView.vue";
import SiteSettingsView from "../views/SiteSettingsView.vue";
import TeamsView from "../views/TeamsView.vue";
import ProfileView from "../views/ProfileView.vue";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      redirect: "/home"
    },
    {
      path: "/home",
      name: "home",
      component: HomeView
    },
    {
      path: "/auth",
      name: "auth",
      component: LoginView
    },
    {
      path: "/login",
      redirect: "/home"
    },
    {
      path: "/contests",
      name: "contests",
      component: ContestsView,
      meta: { requiresAuth: true }
    },
    {
      path: "/teams",
      name: "teams",
      component: TeamsView,
      meta: { requiresAuth: true }
    },
    {
      path: "/profile",
      name: "profile",
      component: ProfileView,
      meta: { requiresAuth: true }
    },
    {
      path: "/contests/:contestId",
      name: "contest-detail",
      component: ContestDetailView,
      props: true,
      meta: { requiresAuth: true }
    },
    {
      path: "/contests/:contestId/scoreboard-wall",
      name: "scoreboard-wall",
      component: ScoreboardWallView,
      props: true,
      meta: { requiresAuth: true }
    },
    {
      path: "/admin",
      name: "admin",
      component: AdminView,
      meta: { requiresAuth: true, requiresAdmin: true }
    },
    {
      path: "/admin/site-settings",
      name: "admin-site-settings",
      component: SiteSettingsView,
      meta: { requiresAuth: true, requiresAdmin: true, requiresSuperAdmin: true }
    }
  ]
});

router.beforeEach(async (to) => {
  const authStore = useAuthStore(pinia);
  authStore.hydrateFromStorage();

  if (authStore.accessToken && !authStore.user) {
    await authStore.syncCurrentUser();
  }

  if (to.name === "auth" && authStore.isAuthenticated) {
    const hasAuthWorkflowQuery =
      typeof to.query.verify_token === "string" || typeof to.query.reset_token === "string";
    if (!hasAuthWorkflowQuery) {
      return { name: "contests" };
    }
  }

  if (to.meta.requiresAuth && !authStore.isAuthenticated) {
    return {
      name: "auth",
      query: {
        redirect: to.fullPath
      }
    };
  }

  if (to.meta.requiresSuperAdmin) {
    const role = authStore.user?.role ?? "";
    if (role !== "admin") {
      return { name: "admin" };
    }
  }

  if (to.meta.requiresAdmin) {
    const role = authStore.user?.role ?? "";
    if (role !== "admin" && role !== "judge") {
      return { name: "contests" };
    }
  }

  return true;
});

export default router;
