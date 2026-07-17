import { useEffect } from "react";

export function DocumentTitle({ title }: { title: string }) {
  useEffect(() => {
    document.title = `${title} · Canvas Video Helper`;
  }, [title]);
  return null;
}
