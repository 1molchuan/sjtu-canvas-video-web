import { Navigate, Outlet, Route, Routes } from "react-router-dom";

import { useAuth } from "./auth/auth_context";
import { AppShell } from "./components/app_shell";
import { DocumentTitle } from "./components/document_title";
import { ErrorNotice } from "./components/error_notice";
import { LoadingState } from "./components/loading_state";
import { CourseVideosPage } from "./pages/course_videos_page";
import { CoursesPage } from "./pages/courses_page";
import { LoginPage } from "./pages/login_page";
import { NotFoundPage } from "./pages/not_found_page";
import { PrivacyPage } from "./pages/privacy_page";
import { VideoPage } from "./pages/video_page";

export function AppRoutes() {
  return (
    <Routes>
      <Route path="/" element={<HomeRoute />} />
      <Route path="/login" element={<Titled title="登录"><LoginPage /></Titled>} />
      <Route path="/privacy" element={<Titled title="隐私与使用说明"><PrivacyPage /></Titled>} />
      <Route element={<ProtectedRoute />}>
        <Route path="/courses" element={<Titled title="课程"><CoursesPage /></Titled>} />
        <Route path="/courses/:courseHandle" element={<Titled title="课程录像"><CourseVideosPage /></Titled>} />
        <Route
          path="/courses/:courseHandle/videos/:videoHandle"
          element={<Titled title="录像轨道"><VideoPage /></Titled>}
        />
      </Route>
      <Route path="*" element={<Titled title="页面不存在"><NotFoundPage /></Titled>} />
    </Routes>
  );
}

function HomeRoute() {
  const { state } = useAuth();
  if (state.status === "checking") return <CheckingSession />;
  if (state.status === "authenticated") return <Navigate to="/courses" replace />;
  if (state.status === "error") return <SessionError />;
  return <Navigate to="/login" replace />;
}

function ProtectedRoute() {
  const { state } = useAuth();
  if (state.status === "checking") return <CheckingSession />;
  if (state.status === "authenticated") return <Outlet />;
  if (state.status === "error") return <SessionError />;
  return <Navigate to="/login" replace state={{ sessionExpired: state.status === "expired" }} />;
}

function CheckingSession() {
  return <AppShell compact><LoadingState label="正在检查登录状态" /></AppShell>;
}

function SessionError() {
  const auth = useAuth();
  if (auth.state.status !== "error") return null;
  return (
    <AppShell compact>
      <ErrorNotice error={auth.state.error} title="无法检查登录状态" onRetry={() => void auth.refresh()} />
    </AppShell>
  );
}

function Titled({ title, children }: { title: string; children: React.ReactNode }) {
  return <><DocumentTitle title={title} />{children}</>;
}
