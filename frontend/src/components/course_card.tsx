import { Link } from "react-router-dom";

import type { Course } from "../api/schemas";

export function CourseCard({ course }: { course: Course }) {
  const metadata = [course.course_code, course.term_name].filter(Boolean).join(" · ");
  return (
    <article className="resource-card course-card">
      <p className="resource-card__kind">课程档案</p>
      <h2>{course.name || "未命名课程"}</h2>
      <p className="resource-card__meta">{metadata || "课程信息暂缺"}</p>
      <Link className="text-link" to={`/courses/${encodeURIComponent(course.id)}`} state={{ courseName: course.name }}>
        查看课程录像 <span aria-hidden="true">→</span>
      </Link>
    </article>
  );
}
