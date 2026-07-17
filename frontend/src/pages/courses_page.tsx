import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

import { createCourseApi, shouldRetryQuery, type CourseApi } from "../api/courses";
import { useAuth } from "../auth/auth_context";
import { AppShell } from "../components/app_shell";
import { CourseCard } from "../components/course_card";
import { EmptyState } from "../components/empty_state";
import { ErrorNotice } from "../components/error_notice";
import { LoadingState } from "../components/loading_state";

export function CoursesPage({ api }: { api?: CourseApi }) {
  const auth = useAuth();
  const courseApi = useMemo(() => api ?? createCourseApi(auth.apiClient), [api, auth.apiClient]);
  const query = useQuery({
    queryKey: ["courses"],
    queryFn: courseApi.getCourses,
    enabled: auth.session !== null,
    retry: shouldRetryQuery,
    retryDelay: 0,
  });
  const actions = (
    <>
      <button className="button button--quiet" type="button" onClick={() => void query.refetch()}>刷新课程</button>
      <button className="button button--quiet" type="button" onClick={() => void auth.logout()}>登出</button>
    </>
  );

  return (
    <AppShell actions={actions}>
      <header className="page-heading">
        <p className="eyebrow">你的 Canvas 权限范围</p>
        <h1>课程档案</h1>
        {auth.session !== null && <p>本次登录有效至 {formatExpiry(auth.session.expires_at)}</p>}
      </header>
      {query.isPending && <LoadingState label="正在获取课程" />}
      {query.error !== null && <ErrorNotice error={query.error} onRetry={() => void query.refetch()} />}
      {query.data?.courses.length === 0 && (
        <EmptyState title="暂时没有可访问的课程" description="Canvas 当前没有返回可访问课程。" />
      )}
      {query.data !== undefined && query.data.courses.length > 0 && (
        <section className="course-grid" aria-label="课程列表">
          {query.data.courses.map((course) => <CourseCard key={course.id} course={course} />)}
        </section>
      )}
    </AppShell>
  );
}

function formatExpiry(value: string): string {
  return new Intl.DateTimeFormat("zh-CN", { dateStyle: "medium", timeStyle: "short" }).format(new Date(value));
}
