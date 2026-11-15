import type { StepStatus } from './domain/steps';

const STATUS_LABEL: Record<StepStatus, string> = {
  pending: '대기',
  running: '진행 중',
  done: '완료',
};

type Props = {
  readonly title: string;
  readonly status: StepStatus;
  /** 제목 줄 오른쪽에 두는 요소(다시 감지 버튼 등) */
  readonly action?: React.ReactNode;
  readonly children?: React.ReactNode;
};

/** 시작 설정의 한 단계. 제목 옆에 진행 상태를 보여 준다. */
function StepSection({ title, status, action, children }: Props): React.JSX.Element {
  return (
    <section className="onboarding-step" data-status={status}>
      <header>
        <h2>{title}</h2>
        <span className="onboarding-status">{STATUS_LABEL[status]}</span>
        {action}
      </header>
      {children}
    </section>
  );
}

export default StepSection;
