type Props = {
  readonly title: string;
  /** 제목 줄 오른쪽에 두는 요소(다시 감지 버튼 등) */
  readonly action?: React.ReactNode;
  readonly children?: React.ReactNode;
};

/** 시작 설정에서 지금 보고 있는 단계의 내용. 제목 옆에 그 단계의 다시 하기 버튼을 둔다. */
function StepSection({ title, action, children }: Props): React.JSX.Element {
  return (
    <section className="onboarding-step">
      <header>
        <h2>{title}</h2>
        {action}
      </header>
      {children}
    </section>
  );
}

export default StepSection;
