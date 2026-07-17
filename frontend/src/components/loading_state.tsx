type LoadingStateProps = {
  label?: string;
};

export function LoadingState({ label = "正在加载" }: LoadingStateProps) {
  return (
    <div className="loading-state" role="status" aria-live="polite">
      <span className="loading-mark" aria-hidden="true" />
      <p>{label}</p>
    </div>
  );
}
