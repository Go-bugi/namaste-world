type Props = {
  readonly canPrev: boolean;
  readonly canNext: boolean;
  readonly error?: string;
  readonly onPrev: () => void;
  readonly onNext: () => void;
};

/** 이전과 다음 버튼. 넘어갈 수 없거나 돌아갈 수 없으면 버튼을 비활성화한다. */
function StepNav({ canPrev, canNext, error, onPrev, onNext }: Props): React.JSX.Element {
  return (
    <footer className="onboarding-nav">
      {error && <p className="onboarding-error">{error}</p>}
      <div className="onboarding-actions">
        <button type="button" disabled={!canPrev} onClick={onPrev}>
          이전
        </button>
        <button type="button" disabled={!canNext} onClick={onNext}>
          다음
        </button>
      </div>
    </footer>
  );
}

export default StepNav;
